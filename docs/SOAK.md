# Soak test with headless bots

`net-tool netsim soak` (T08-029; `tools/net-tool`, `net_tool::soak`) runs N scripted clients and one server for M minutes of **virtual time** over a conditioned link (`ss2_net::netsim`, presets in
`docs/NET_TOOL.md`), using the **real session layer** (`ss2_net::session`) and the **real input stream** (`ss2_net::inputstream`), and **checks invariants** instead of printing impressions. It exits 0 when they hold, **1** when
one does not (the failures are listed as `problem:` lines), 2 for a configuration that makes no sense.

```
net-tool netsim soak --preset bad_wifi --bots 16 --minutes 600 --churn-seconds 30
```

## What is simulated, and what is not

Every bot joins the server (staggered), then sends one input per tick stamped a few ticks ahead of its estimate of the server's tick, in the redundant packets of `InputSender`; the server validates, buffers and takes one input per bot
per tick (`InputBuffer`) and sends back acknowledgements; keepalives and timeouts run as they would. Optionally one bot at a time **leaves and rejoins** (`--churn-seconds`). Time is virtual (30 Hz by default), so thirty minutes of
eight bots take about a second and ten hours of sixteen bots with churn about five minutes (debug build); the same seed gives the same report.

It is **not a game soak**. There is no simulation, no snapshot stream, no prediction, no real transport and no sockets in the repository yet: the "game" is a number per tick and the server's state is what it applied. It soaks **the
protocol modules**. Bandwidth is computed from a **size model** (a session message 24 bytes, an input packet 4 + 12 bytes per frame, an acknowledgement 12 bytes), because the input and acknowledgement messages have no wire encoding yet.

## What a run checks

`SoakReport::problems()` is empty only if: every bot is in the session at the end and **the server agrees on the count**; no bot failed to get in; **no connection was lost to a timeout or a refusal** (a bot that leaves on purpose is not one);
**every input the server applied as received is exactly the input the bot sent for that tick** (no wrong, duplicate or reordered application; a deliberate-fault test proves the check notices); a well-behaved client earns no
`conflicting`, `too far ahead` or `rate-limited` frames; and the share of ticks with **no input at all** stays under `--max-missing` (default 20 %). It also reports (without failing): the ticks applied as received, **repeated**
(the last input held for a missing tick, up to five ticks) and missing, the stale and duplicate frame counts, and the bandwidth.

## What it found

1. **A bot's idea of the server's tick is a round trip behind, so `--lead-ticks` must cover the round trip, the jitter and one more tick.** A bot learns the server's tick from the welcome; without clock synchronisation (`ss2_net::timesync`)
   its estimate is behind by the one-way delay, and its input then needs another one-way delay to arrive. With the default lead of 6 ticks (200 ms at 30 Hz) a link of 60 ms each way with 20 ms of jitter lost 4.7 % of ticks and had
   29 % repeated; 300 ms each way (satellite) lost **all** of them, and `--lead-ticks 24` (25 for the satellite preset) fixes it. The real client must use the clock synchronisation and a lead derived from the measured round trip, not a
   constant: the soak is the test that will show whether it does.
2. **The session layer dropped players whose game traffic was flowing.** Liveness was refreshed only by session messages (keepalives every 2 s against a 10 s timeout, so five in a row must be lost): at **30 % loss each way, all eight bots were dropped within thirty minutes**
   although input packets were arriving all the time. Fixed by two small methods, `SessionServer::heard_from(peer, now)` and `SessionClient::heard_from_server(now)`, that the integration calls for every datagram of any kind; with them the same run
   keeps every bot (`--session-liveness-only` reproduces the old behaviour). Until the transport (T08-004) calls them, the problem is still there.
3. **Redundancy works:** with three frames per packet, 10 % loss each way cost essentially no ticks (below 1 % missing) once the lead was right, and a wrong input was **never** applied in any run above (about 20 million bot-ticks in all, mismatched 0).

## Results

Thirty minutes, eight bots, lead 6 (debug build, about 1.5 s each):

| link | applied as received | repeated | missing | result |
|---|---|---|---|---|
| perfect, lan, broadband | 100 % | 0 | 0 | passed |
| mobile (50 ms, 1 % loss) | 95.5 % | 4.5 % | 0.0005 % | passed |
| bad_wifi (30 ms, 40 ms jitter, 5 % loss, reordering) | 97.8 % | 2.2 % | 0 | passed |
| satellite (300 ms) | 0 | 0 | 100 % | **failed** (lead too short); with `--lead-ticks 25`: passed |
| 40 ms, 30 / 50 / 70 % loss, session liveness only | | | 23 to 46 % | **failed**: every bot dropped, or never got in |

Ten hours, sixteen bots, bad_wifi with one bot leaving and rejoining every 30 seconds: 1,199 leaves and 1,215 joins, the server and the bots agreed at the end, no false disconnects, 9.7 % of ticks without input (lead 6 is tight for that link), mismatched 0.
The bandwidth by the size model is about 4 to 10 kbit/s up and 3 kbit/s down per bot; it is **not a measurement**.

## Not covered (follow-up T08-029b)

- **A game**: state hashes compared between a server simulation and bot copies (the desync detector the roadmap line names), prediction and reconciliation, snapshots. They need the simulation (T08-002, T08-003 integration, T08-007, T08-009).
- **A real transport and real sockets**, and bots as separate processes against a dedicated server (`ss2-server`, T08-020); the real encoded sizes.
- **Memory and leak detection**: the run does not measure the memory of the server or the bots (the bounded tables are tested in their own modules).
- **Nightly scheduling**: nothing runs it in CI (Actions jobs do not start at the moment; T12-016 is the soak farm that would).
- **Hostile bots** (floods, malformed packets): those are the fuzz targets and the unit tests of each module; the bots here are well behaved.
- **Time synchronisation** (`ss2_net::timesync`) in the bot, which would replace the fixed lead.
