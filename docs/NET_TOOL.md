# net-tool: replays and the network simulator on the command line

`tools/net-tool` (T11-010) is a command-line tool for two things that exist without a game: **replay files** (`docs/REPLAY.md`) and the **network simulator** (`ss2_net::netsim`). Run it with
`cargo run -p net-tool -- <command>`. It needs no game data.

Exit codes: **0** success · **1** `replay verify` found a replay that is cut short or damaged after its header · **2** an error (a file that cannot be read or is not a replay, an unknown
preset, a value that makes no sense).

## Replays

| Command | What it does |
|---|---|
| `replay inspect FILE` | The header (build, map, tick rate, players, seed, protocol, schema hash, notes), how many records of each kind, inputs per player, which ticks have snapshots, the tick range and its length at the tick rate, and **how the file ended**: `complete`, or `TRUNCATED at byte N: <reason>` (never finished, cut inside a record, a checksum failure, ...). Exit 0 whenever the header is readable. |
| `replay verify FILE` | `ok: complete, N records, M bytes` and exit 0; or the report and exit 1 if the file is cut short or damaged; exit 2 if it is not a replay. For scripts and CI. |
| `replay dump FILE [--from N] [--to N]` | One line per record in file order, ticks `from` to `to` inclusive: `tick 3 input player 0 4 bytes`, `tick 3 event 2 bytes`, `tick 0 snapshot 32 bytes`, `tick 3 state-hash 0x0000000000000bb8`. Payloads are not shown (the game defines what is in them). |

A truncated file still gives the records before the cut, so a crashed session's recording can be looked at.

## The network simulator

`netsim presets` lists the named links (`perfect`, `lan`, `broadband`, `mobile`, `bad_wifi`, `satellite`, `acceptance_100ms_2pct`). `netsim run` sends a **steady one-way stream of packets** through one link and
reports what the receiver saw:

```
$ net-tool netsim run --preset bad_wifi --seed 5 --packets 1000
sent: 1000 packets of 100 bytes, one every 16.667 ms
received: 952 (95.20 %)
lost on the wire: 48 (4.80 %)
dropped by a full queue: 0 (0.00 %)
duplicates received: 0
out of order: 158
latency (ms): min 30.004 · mean 49.867 · p50 49.045 · p95 68.261 · p99 73.250 · max 97.312

$ net-tool netsim run --preset perfect --bandwidth-kbytes 20 --queue-bytes 1000 --interval-us 1000 --packets 1000
...
received: 210 (21.00 %)
dropped by a full queue: 790 (79.00 %)
latency (ms): min 5.000 · mean 53.390 · p50 55.000 · p95 55.000 · p99 55.000 · max 55.000
```

(The second is 100-byte packets every millisecond, 100 kB/s, into a 20 kB/s link with room for 1 kB: most are dropped, and the ones that get in wait up to 50 ms in the queue.)

Options: `--preset NAME` (default `broadband`), `--seed N` (default 1; **the same seed gives the same report, byte for byte**), `--packets N` (default 1000, at most 5,000,000), `--size BYTES` (default 100),
`--interval-us N` (default 16667, 60 packets per second; 0 sends all at once), and overrides applied on top of the preset: `--latency-ms`, `--jitter-ms`, `--loss-pct`, `--duplicate-pct`, `--reorder-pct`,
`--bandwidth-kbytes` (kilobytes of 1000 bytes per second), `--queue-bytes`. Reordering on a link with no reorder delay gets 30 ms so that it has an effect.

What the numbers mean: **received** counts packets that arrived at least once; latency is that of the **first copy** of each; **out of order** counts packets that arrived after a packet sent later had
already arrived (jitter larger than the packet spacing causes this by itself, which is why `bad_wifi` shows far more than its 2 % of deliberate reordering); **duplicates received** counts second copies.
Percentiles are the nearest rank.

## How it was tested

Twenty-one unit tests and seven end-to-end tests (the real binary), written before the code. The unit tests cover: the replay report (counts, per-player inputs, keyframes, the rendered text, a cut file, damaged and
foreign files), the record dump and its inclusive range, the clock text (minutes, rounding), percentiles, the percent-to-ppm conversion (including values that floating point puts just under a whole number), the presets, the overrides,
and scenarios: a perfect link, a fixed latency, jitter within bounds, loss that adds up (`received + lost + queue drops = sent`), total loss, duplicates and reordering seen by the receiver, a tail-drop queue,
the same seed twice and another seed, and refused scenarios (no packets, too many, empty packets, an invalid link). The end-to-end tests run every command and the exit codes. **Mutation checks:** fifteen deliberate breakages, of which
four survived the first tests (the minute roll-over of the clock, rounding instead of cutting the ppm, the percentile rank, and the "newest packet so far" rule of the out-of-order count), so I added tests and extracted the out-of-order counter into
a function that can be tested on a given arrival order. Two more changes are equivalent and are not tested: a boundary in that counter (an index never arrives twice as a first copy) and the zero-bandwidth guard
in the overrides (the link's own validation refuses zero as well).

## Not covered (follow-up T11-010b)

- **Record and play back a session.** There is no simulation to record yet; the playback driver is T08-025b. This tool reads and checks files, it does not make them.
- **A scenario that looks like the game's traffic** (many small packets per tick from several clients, a bursty snapshot stream). Today it is one steady one-way stream; a two-way run (`Duplex`) and a round-trip report
  are not built. The **socket-level conditioner** that applies these presets to real UDP traffic is T08-017b.
- **Machine-readable output** (JSON) for scripts and the progress dashboard (T11-012); the text is for people, and `replay verify` has an exit code for scripts.
- **Scenario files** (a TOML that describes a run, to commit next to a test); today the options are the interface.
- It is **not in the release archives** (`docs/PACKAGING.md` lists what ships); it is a development tool.
