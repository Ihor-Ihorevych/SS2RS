# Transport spike (T08-005)

Throwaway benchmark behind `docs/rfcs/0006-network-transport.md`. On loopback it measures plain UDP (blocking threads and tokio)
against QUIC (`quinn` 0.11 with `rustls` and `ring`): 64-byte ping-pong round trips, the QUIC handshake, the maximum datagram size and
the wire bytes per application datagram. Then it puts a small UDP proxy that delays and drops packets between a QUIC client and
server and sends 60 messages a second of 200 bytes for 15 s, once as unreliable datagrams and once on one reliable stream, at 0 %,
5 % and 15 % loss each way with 50 ms each way.

```
cd spikes/transport
cargo run --release       # about 2 minutes, mostly the three 15 second lossy runs
```

It has its own `[workspace]` and `Cargo.lock`: not part of the main workspace, CI or `cargo deny`, and nothing in it ships. Delete it
once the transport crate exists. The proxy uses tokio timers, which on Windows have a granularity of about 15 ms, so "50 ms each way"
measures about 62 ms: compare the rows with each other, not with the configured delay.
