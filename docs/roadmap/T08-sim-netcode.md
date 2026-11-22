# T08 — Simulation Core & Co-op Netcode (`ss2-sim`, `ss2-net`)

**Co-op is first-class.** Architecture: server-authoritative fixed-step simulation; clients send
input commands; server sends snapshots/deltas; remote entities are interpolated; the local player is
predicted and reconciled; hitscan uses server-side lag compensation. Single-player = a local
server + one client over an in-process loopback transport (same code path = co-op bugs surface early).
Not goals: original protocol compatibility, lockstep determinism.

## Sim core
- [x] **T08-001** · **SPIKE** ECS choice: custom minimal ECS vs `hecs`/`legion`-style crate; requirements: stable entity IDs w/ generations, per-component replication flags, reflection hooks, cheap snapshot of state; RFC: `docs/rfcs/0005-ecs-choice.md` (in review; proposes `hecs` behind an `ss2-sim` facade, with benchmark in `spikes/ecs/`) · `M` · `P0`
- [ ] **T08-002** · Entity/class model: class registry, property reflection compatible with `.ep`/`.wld` records, spawn-from-record, lifecycle (spawn/init/tick/destroy), parenting · `L` · `P0` · Deps: T08-001, T01-014/026
- [ ] **T08-003** · Fixed-timestep sim loop: tick rate from T01-022, pause, time scale, slow-mo, deterministic ordering of systems, phases (input→AI→move→physics→damage→cleanup→replicate) · `M` · `P0`
- [ ] **T08-004** · Server/client split: `ServerSim`, `ClientSim`, shared game code, loopback transport for SP, listen server (host plays) and dedicated server · `L` · `P0`
- [ ] **T08-028** · Event bus (gameplay events, triggers I/O) with replication-aware events · `M` · `P1`

## Transport & sessions
- [x] **T08-005** · **SPIKE** transport: QUIC (`quinn`) vs custom UDP (renet/laminar-style) vs WebRTC-ish; need: unreliable + reliable-ordered channels, MTU/fragmentation, encryption, NAT-friendliness, small overhead, and nothing that blocks a later Android build; RFC: `docs/rfcs/0006-network-transport.md` (in review; proposes QUIC datagrams for gameplay and one stream for bulk, behind an `ss2-transport` trait; benchmark in `spikes/transport/`) · `M` · `P0`
- [ ] **T08-006** · Connection lifecycle: handshake, protocol/version/mod-hash check, session password, auth token, timeouts, graceful disconnect, kick/ban · `L` · `P0`
- [ ] **T08-019** · Session discovery: LAN broadcast, direct IP, invite code via lightweight relay/rendezvous (optional master server later), NAT traversal/hole punching · `L` · `P1`
- [ ] **T08-020** · Dedicated server binary: headless, config file, console/RCON-lite, log rotation, metrics endpoint, Docker image · `M` · `P1`
- [ ] **T08-027** · Encryption/auth if transport lacks it (DTLS/noise), replay protection · `M` · `P2`

## Replication & prediction
- [ ] **T08-007** · Replication framework: per-component/field replication attributes, delta compression vs last-acked snapshot, quantization (positions, angles), relevancy/interest management, priority & bandwidth budget (hordes!) · `XL` → split · `P0`
- [x] **T08-008** · Snapshot buffer + interpolation/extrapolation for remote entities; adaptive interp delay; teleport/despawn handling: `ss2_net::snapshots` (`docs/SNAPSHOTS.md`); the snapshot protocol and transport that feed it are T08-007 and T08-005 · `L` · `P0`
- [x] **T08-011** · Input command stream: tick-stamped, batched with redundancy, server-side validation/rate limiting, input buffering on server: `ss2_net::inputstream` (`docs/INPUTSTREAM.md`); wire format T08-030, session T08-006 · `M` · `P0`
- [ ] **T08-009** · Client-side prediction + reconciliation of the local player using T07-005 controller; smooth error correction; misprediction metrics · `XL` → split · `P0` · Deps: T07-005, T08-011
- [ ] **T08-010** · Server-side lag compensation for hitscan/melee (history of hitboxes, max rewind window, fairness limits) · `L` · `P1`
- [x] **T08-012** · Reliable events (explosions, pickups, chat, audio events) with dedupe across redundant packets and late-join suppression: `ss2_net::events` (`docs/EVENTS.md`); RPC calls and the event bus are T08-028 · `L` · `P0`
- [ ] **T08-015** · Authority for physics objects/projectiles/AI: server simulates, clients interpolate; client-side effect prediction for local weapon fire (muzzle flash, tracer) · `L` · `P1`
- [x] **T08-016** · Time sync: clock offset/RTT estimation, tick alignment, jitter buffer delay: `ss2_net::timesync` (`docs/TIMESYNC.md`); the transport and the snapshot buffer that use it are T08-005 and T08-008 · `M` · `P0`
- [ ] **T08-013** · Join-in-progress: full-state snapshot streaming, level-load sync, spawn rules, late-joiner event suppression · `L` · `P0`
- [ ] **T08-014** · Reconnect & drop-in/out handling; (optional later) host migration · `L` · `P2`

## Co-op game-layer rules (with T09)
- [ ] **T08-023** · Co-op rules: respawn/checkpoint sync, shared level progress, enemy scaling by player count, friendly fire option, player ghosting at transitions, level-transition sync · `L` · `P1` · Deps: T01-037
- [ ] **T08-024** · Spectator/follow-cam for dead players · `M` · `P2`
- [ ] **T08-022** · Save/load as serialization of sim state (own format) + co-op saves, checkpoints, quick-save rules · `L` · `P1`

## Tooling, tests, ops
- [x] **T08-017** · Network simulator (latency, jitter, loss, duplication, reorder, bandwidth cap with a tail-drop queue): `ss2_net::netsim`, deterministic and in-process; the socket-level conditioner is T08-017b and the scenario runner is T11-010 · `M` · `P0`
- [ ] **T08-017b** · Socket-level network conditioner: a UDP proxy tool that applies the `ss2_net::netsim::LinkConfig` presets to real traffic (it needs a real clock, so it lives in a tool crate or `xtask`, not in the sim-safe `ss2-net`); lets a real client and server be tried at 100 ms and 2 % loss · `M` · `P1` · Deps: T08-005
- [ ] **T08-029** · Headless bot clients for soak tests (N bots × M minutes); assertions on desync/divergence/bandwidth · `L` · `P1`
- [~] **T08-018** [INPRG @Ihor-Ihorevych/agent · branch `T08-018-net-stats` · 2026-11-22] · Net profiling: bytes/tick per component, RTT/loss graph, prediction error HUD (`net_graph`), Tracy spans · `M` · `P1`
- [ ] **T08-021** · Server validation & anti-cheat basics: sanity bounds on input, rate limits, authority checks; threat model doc · `M` · `P2`
- [ ] **T08-025** · Replay recording (inputs + snapshots) and playback for bug reports · `L` · `P2`
- [ ] **T08-026** · Prediction error test across x86_64/aarch64 clients vs server (T07-026) · `M` · `P1`
- [x] **T08-030** · Protocol versioning & compatibility policy, schema hash, changelog for wire format: `ss2_net::protocol` and `docs/PROTOCOL.md` (there is no wire format yet, so the changelog starts empty) · `S` · `P1` · 🟢
- [ ] **T08-031** · Mobile networking: Wi-Fi↔cellular handoff, background/suspend reconnect, battery-aware tick/snapshot rate for clients · `M` · `PHASE 2`
- [x] **T08-032** · Documentation: netcode architecture guide: `docs/NETCODE.md` (model, layers, what exists, a tick step by step, rules for gameplay code); the replication and prediction sections are provisional until T08-007 and T08-009 · `M` · `P1`
