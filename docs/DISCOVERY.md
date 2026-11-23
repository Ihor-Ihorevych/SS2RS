# Session discovery: LAN scan, direct addresses, invite codes

`ss2_net::discovery` (T08-019) is how a player finds a game to join. It is **pure logic over datagrams**, like the rest of `ss2-net`: no sockets, no clock, no randomness (the caller passes time, a nonce and the source
address). The socket that broadcasts the query and receives the replies is the transport's (RFC 0006); it does not exist yet.

## LAN scan

1. The client's `Scanner::start(now_us, nonce, timeout_us)` returns a **query** to broadcast and remembers the nonce.
2. A server's `Announcer::handle(now_us, from, datagram)` answers a query with an **info** message: name, map, game mode, players and slots, the **game port**, the protocol version and **schema hash**, the hash of its mods, and
   whether a password is needed. It echoes the query's nonce. Anything else gets no answer (`Handled::Ignored(reason)`).
3. `Scanner::on_datagram(now_us, from, datagram)` accepts a reply only if it is an info message, **echoes the scan's nonce**, and arrives within the timeout (`Rejected::{NotScanning, Malformed, WrongNonce, TooLate, ListFull}`).
   The entry is keyed by **the address the reply came from, with the game port from the reply**, so a reply cannot make the list point at somebody else's address. It records the round trip time and whether this build can talk to the
   server (`ss2_net::protocol::check_handshake`: `Compatible`, `VersionMismatch` or `SchemaMismatch`).
4. `ServerList::for_display()` orders the list: servers this build can join first, then those with a free slot, then the fastest, then by address. `expire(now_us, max_age_us)` drops servers not heard from. The list holds at
   most 256 servers (a known server may still update when it is full).

The info fields are checked on both ends: a name that is not empty, at most 32 bytes, no control characters; map 48 and mode 24 bytes; `players <= max_players <= 64`; a non-zero port.

## Not a reflector

A UDP service that answers a **spoofed** source address with more bytes than it received is a weapon against a third party. So:

- a query must be **padded to 256 bytes**, and the largest reply that can be encoded is smaller (a test encodes the worst case); **a reply is never longer than the query that caused it**, so there is no amplification;
- an `Announcer` limits answers **per source address** (default: a burst of 4, then 2 per second) and **overall** (a burst of 200, then 100 per second), with integer token buckets; a request refused by the overall limit does not use up
  the source's token; the table of sources is **bounded** (1,024; the one seen longest ago is forgotten);
- unpadded queries, other traffic and info messages get **no answer at all**.

## Direct addresses and invite codes

- `parse_direct("192.168.1.5")`, `"host:27015"`, `"[::1]:27015"`, `"::1"`: an IP address or a host name with a port (default 27015). Names are lower-cased and checked (labels of 1 to 63 letters, digits and inner dashes, at most
  253 characters); a name that looks like an IPv4 address but is not one (`999.1.1.1`, `1.2.3`) is refused; ports are 1 to 65535 in at most five digits. **Resolving a host name is the caller's job.**
- `encode_invite(addr)` makes a short, typeable code for an IP address and port: **`XXXX-XXXX-XXXX-XXX`** for IPv4 (9 bytes, 15 characters), 34 characters for IPv6. The alphabet is Crockford's base 32 (no `I`, `L`, `O`, `U`); `decode_invite`
  ignores case, dashes and spaces and reads `O`, `I`, `L` as `0`, `1`, `1`. A code has a version, the address kind, the address, the port and a **16-bit checksum**: it is refused for a wrong length, a character outside the alphabet, a bad
  checksum, an unknown version or kind, non-zero padding bits, port 0 or an unspecified address. For the one address I tried, none of the 420 possible single-character typos was accepted as any address; in general a random error passes
  the checksum with probability 2^-16, so **a code is a typo guard, not a security measure**.

## What it does not do

- **Authenticate who answered.** Anyone on the LAN can answer a scan, and an invite code carries no key. The list is a convenience; the **connection pins the host's key or certificate** (RFC 0006, `docs/THREAT_MODEL_NET.md`), it does not
  trust the list.
- **The relay or rendezvous server** for invite codes over the internet, **NAT traversal and hole punching**, and a master server: none exists. The codes here are addresses, so they work for a host that can be reached directly.
- **A socket.** Broadcasting, the discovery port number, IPv6 multicast and platform quirks (Windows firewalls, Android multicast locks) are the transport's and untested.
- **A screen.** The lobby UI is T10-018.
- **Server name hygiene beyond control characters** (profanity, impersonation of another server's name): a name is shown as the server sent it.

## How it was tested

Twenty-five tests, a fuzz target and mutation checks. Messages: round trips including the worst case, the reply that is smaller than the smallest query, every field limit, every refusal reason, every prefix of a reply refused, a datagram of exactly the
maximum size. Answering: the reply and its nonce, short queries and other traffic and info messages ignored, the per-source limit (the burst, the refill, other sources unaffected, the burst not exceeded after a long quiet time), the overall limit and
that a refused source keeps its token, the bounded source table. Scanning: the nonce, the timeout boundary, a new scan ignoring old replies, compatibility marking and display order, the bounded list, expiry and its boundary, the game port. Addresses
and codes: every form, thirty-odd nonsense inputs, the exact limits of host names and ports, round trips for IPv4 and IPv6, loose reading, every single-character typo, version and padding, well-formed codes with wrong content. **Thirty-five
deliberate breakages each fail a test; eleven survived my first tests** (the expiry boundary, the port and host length limits, an invite of the wrong length with a good checksum, port 0 or an unspecified address in a code, an unknown address kind), so
I added tests for them. One more (a length cap that a later exact length check makes redundant) is equivalent. The fuzz target `discovery_message` (see `docs/FUZZING.md`) checks that no reply is ever longer than its query, that the list
stays bounded and that a code that reads encodes back to the same address: **60 seconds, 120,202 runs, no finding**; a smoke test, not coverage, and not yet run in CI.

## Not covered (follow-up T08-019b)

- **The sockets**: broadcast on the LAN, the discovery port, and answering from the real server (the dedicated server T08-020 has no discovery yet).
- **The relay or rendezvous server, NAT traversal, hole punching, a master server** (the roadmap line lists them; each is its own piece).
- **Invite codes that carry the host's key fingerprint** (RFC 0006 pins it "learned from the invite or the discovery step"): a short code cannot hold a 32-byte fingerprint, so an invite *link* or a rendezvous lookup is needed.
- **The lobby UI** (T10-018) and **a server browser's refresh policy** (how often to scan, when to expire).
