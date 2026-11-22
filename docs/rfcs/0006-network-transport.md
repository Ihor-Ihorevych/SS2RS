# RFC 0006: The network transport

- **Number:** 0006 (`docs/rfcs/0006-network-transport.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review
- **Approvers:** owner of `ss2-net` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T08-005 (this spike); unblocks T08-006 (connection lifecycle), T08-004 (server/client split), T08-019 (session discovery), T08-020 (dedicated server), T08-027 (encryption), T08-031 (mobile networking)

## Summary

Use **QUIC through `quinn`**: **unreliable datagrams for all gameplay traffic** (inputs, snapshots, events) and **one reliable stream
only for bulk transfers** (the full state a client gets when it joins, level data). Put it behind a small `Transport` trait in a **new
crate** (`ss2-transport`), so that `ss2-net` (which must not read a clock or use a socket) stays pure and the choice can be replaced.
The fallback is a custom UDP protocol with a Noise handshake (`snow`) or the `renet`/`renet_netcode` pair.

This is the output of a time-boxed spike: a throwaway benchmark (`spikes/transport/`, outside the main workspace) that measured QUIC
against plain UDP on loopback and put QUIC through a lossy, delaying proxy. **Loopback shows the cost of the stack; the proxy shows the
behaviour under loss; neither is a real network**, and the recommendation says where the evidence stops.

## Motivation

Everything in `ss2-net` so far (the input stream, the reliable events, the snapshot buffer, the time sync, the protocol version check) is
written over an **unreliable datagram** pipe: each module repeats what is unconfirmed instead of relying on the transport to resend. What
the game still needs from a transport is: an **authenticated, encrypted** datagram pipe with connection setup and teardown, a way to send
**large reliable blobs** (the state of the world for a late joiner), handling of the MTU, and nothing that blocks a later Android build.
The connection lifecycle (T08-006), the dedicated server (T08-020) and session discovery (T08-019) cannot start until this is chosen.

## Requirements and what the candidates give

| Requirement (from the roadmap line) | QUIC with `quinn` (measured here) | custom UDP, `renet`-style (not measured) |
|---|---|---|
| Unreliable channel | **QUIC datagrams** (RFC 9221): behave like UDP, see below | native |
| Reliable-ordered channel | streams: **slow under loss** (below) | whatever we build |
| MTU and fragmentation | datagrams up to **1,162 bytes** here (the connection reports it; it can grow with path MTU discovery); larger data goes on a stream | we write it |
| Encryption and authentication | **TLS 1.3 built in**; needs a certificate or a pinned key (below) | we write it (Noise via `snow`, or the netcode handshake of `renet_netcode`) |
| NAT-friendliness | UDP; hole punching still needs a rendezvous (T08-019); QUIC connection ids allow migration (RFC 9000), not tested | UDP; same |
| Small overhead | **about 40 bytes per packet** beyond the payload (below) | a header of our choosing, a few bytes plus the cipher tag |
| Nothing that blocks Android | `ring` compiles C; **not verified** (no NDK here); async runtime (`tokio`) or the sans-IO core | pure Rust apart from the crypto crate |

## Measurements

Windows 11, Rust 1.99.1, 12th Gen Intel i5-12450HX, release build, `quinn` 0.11.12 with `rustls` and `ring`, three runs of the whole program (ranges are the spread).

### Loopback, 64-byte ping-pong, 20,000 round trips (microseconds)

| | mean | p50 | p99 | p99.9 |
|---|---|---|---|---|
| plain UDP, blocking threads | 16 to 18 | 15 to 16 | 43 to 54 | 111 to 145 |
| plain UDP through tokio | 24 to 27 | 22 to 24 | 54 to 80 | 139 to 209 |
| QUIC unreliable datagram | 31 to 34 | 28 to 30 | 87 to 97 | 201 to 213 |
| QUIC reliable stream | 33 to 35 | 29 to 31 | 97 to 101 | 191 to 212 |

- QUIC costs **about 6 to 8 microseconds more per round trip** than the same socket through tokio (the median and the mean): nothing next to a network round trip of tens of milliseconds.
- The **handshake** (TLS 1.3) took about **1.2 ms** of CPU on loopback and one round trip on a real path.
- **Wire overhead:** the client sent 20,521 UDP datagrams and 2,136,500 bytes for 20,500 application datagrams of 64 bytes: **104.2 bytes of UDP payload per 64-byte message**, so about **40 bytes of QUIC header, authentication tag and frame headers** (the acknowledgements ride on the data packets in a ping-pong). Plain UDP has none, but a custom protocol would still need its own header and, encrypted, an authentication tag, so the real saving is smaller than 40 bytes. At 60 packets per second per direction 40 bytes are about 2.4 KB/s.

### Under loss and delay (a UDP proxy; 60 messages per second of 200 bytes, 15 s, 900 messages)

50 ms each way configured; the proxy's timers on Windows round to about 15 ms, so the baseline row reads 63 ms. **Compare rows, not absolute values.**

| Loss each way | Datagrams delivered | datagram delay p50 / p99 (ms) | Stream delivered | stream delay p50 / p99 (ms) |
|---|---|---|---|---|
| 0 % | 900 of 900 | 62.6 / 64.7 | 900 of 900 | 62.6 / 64.7 |
| 5 % | 858 and 860 of 900 (95.3 % and 95.6 %) | 63.1 and 63.2 / 156.5 and 172.6 | 900 of 900 | **77.8 and 93.9 / 406 and 607** |
| 15 % | 798 and 755 of 900 (88.7 % and 83.9 %) | 93 and 109 / 201 and 231 | 900 of 900 | **281 and 639 / 750 and 2,902** |

(Two runs per loss level.) What this shows:

- **Datagrams behave like UDP**: the loss shows up as lost messages, not as delay, so the redundancy schemes of `ss2-net` apply unchanged. The delay of the surviving messages rises at higher loss (p99 about 160 ms at 5 %): I could not separate QUIC's congestion control and pacing from the proxy's timer noise, so treat it as an upper bound.
- **The reliable stream is unusable for gameplay state**: every message arrives, but behind the lost ones (head-of-line blocking): the 99th percentile is 400 to 600 ms at 5 % loss and seconds at 15 %. That is why **gameplay stays on datagrams** and the stream is for **bulk** only.

## Detailed design

### The transport trait (in a new crate `ss2-transport`)

`ss2-net` is sim-safe: no wall clock, no sockets. The transport needs both, so it lives next to it:

```rust
pub trait Transport {
    /// An unreliable datagram of at most `max_datagram_size()` bytes. Never blocks; may be dropped.
    fn send_unreliable(&mut self, peer: PeerId, data: &[u8]) -> Result<(), SendError>;
    /// A large reliable message, delivered in order: for bulk transfers only (join state, level data).
    fn send_bulk(&mut self, peer: PeerId, data: Vec<u8>) -> Result<(), SendError>;
    /// Everything that happened since the last call: datagrams, bulk messages, connections, disconnections.
    fn poll(&mut self) -> Vec<TransportEvent>;
    fn max_datagram_size(&self, peer: PeerId) -> usize;
}
```

Implementations: a **loopback/in-process one** for single player and tests (it can be driven through `ss2_net::netsim`), and **QUIC**. The traffic of `ss2-net` (inputs, snapshots, events) is serialized into datagrams of at most `max_datagram_size()` (about 1,100 bytes to be safe); the snapshot code must split a snapshot to fit, which QUIC datagrams do not do for us.

### Running QUIC in a loop-driven engine

`quinn` needs the `tokio` runtime (14 crates in its tree). The plan: a **dedicated network thread** owns the runtime and the endpoint and talks to the game thread through channels carrying `TransportEvent`s and outgoing messages. If that bridge proves awkward, `quinn-proto` (the sans-IO state machine inside `quinn`: it takes bytes and a time and returns bytes and timers) can be driven directly from our loop with our own UDP socket and clock; not tried here.

### Authentication without a certificate authority

A listen server runs on a player's machine, so there is no certificate from a CA. The host generates a **self-signed certificate (or a keypair) per session**; the client **pins its fingerprint**, which it learns from the invite, the LAN broadcast or the join code (T08-019) over a channel the player trusts. A session password (T08-006) is checked **inside** the encrypted connection. A dedicated server can use a real certificate or the same pinning. This is the part of the design that is mine to check against the security review of T08-021; it is not implemented or measured.

## Alternatives considered

- **Custom UDP, `renet`/`laminar`-style.** Smallest overhead and total control of pacing and redundancy, and a small dependency tree: the tree of `renet` 2.0.1 has **4 crates** (itself included) and that of `renet_netcode` 2.0.1 (the encrypted netcode transport) **27**; `laminar` 0.5.0 has 17, `quinn` with `rustls` and `ring` 54 (of which `tokio` and its dependencies are 14). Against it: we would own the handshake, the encryption, MTU discovery, congestion control for the bulk transfers and the connection lifecycle, and a mistake in the cryptography is a security bug. **`snow` 0.10.0** (Noise, Apache-2.0 or MIT, MSRV 1.85) would remove the cryptographic design but not the protocol. This is the **fallback** if the QUIC dependency or its Android build proves too heavy. I did not benchmark these crates and did not read their documentation beyond the metadata; their features and maintenance are **not assessed**.
- **Plain TCP.** Rejected for gameplay: all of it has the head-of-line blocking measured above, with no unreliable channel.
- **WebRTC-style (`webrtc-rs`).** Would give browser clients and NAT traversal, but the web is out of scope until Phase 2 (T03-029 is a feasibility note only) and its dependency tree and complexity are far larger. **Not evaluated.**
- **Raw UDP with our own thin framing and no encryption.** Cheapest, but encryption and authentication are requirements (T08-027), and adding them later to a deployed protocol is harder than choosing a transport that has them.

## Compatibility and migration

New crate `ss2-transport` (the QUIC dependency lives only there); `ss2-net` is unchanged. `cargo deny` must be run when the dependency is added (`quinn`, `rustls`, `ring`/`aws-lc-rs` and their licences: `quinn` is MIT or Apache-2.0; the licence list of `ring` and `rustls-webpki` was not checked against `deny.toml` here). The spike crate is not part of the workspace or CI and can be deleted when the transport exists; its `Cargo.lock` is committed so that the numbers can be reproduced. If the transport is swapped, only `ss2-transport` changes; the version handshake (`docs/PROTOCOL.md`) and the modules of `ss2-net` do not.

## Open questions

- **Does QUIC build for Android?** `ring` compiles C and needs the NDK toolchain, which the Android build needs anyway; not verified. *Decides:* whoever starts T03-024 (Android packaging); a failure moves this to the fallback.
- **How does QUIC's congestion control treat datagrams on a real, congested link?** The proxy suggests datagram loss tracks the link's loss, but the proxy has no bandwidth limit. A run through `ss2_net::netsim`-style conditions with a bandwidth cap (the socket-level conditioner, T08-017b) would answer it. *Decides:* the owner of `ss2-net`, before T08-006 is finished.
- **Connection migration** (Wi-Fi to cellular, T08-031): defined by QUIC, not tested through `quinn`'s API. *Decides:* Phase 2.
- **Is the extra 40 bytes per packet and the tokio thread worth it against a custom protocol?** I think so for the encryption and the maintained loss handling, but that is a judgment about the team's capacity to maintain a protocol, not something measured. *Decides:* the approving maintainers.
- **Maintenance of `quinn`.** Version and licence were checked; its release activity was not. *Decides:* the approving maintainers.
