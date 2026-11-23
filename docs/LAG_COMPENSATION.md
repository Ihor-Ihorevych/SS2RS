# Lag compensation

`ss2_physics::lagcomp` (T08-010) lets the server judge a hitscan or melee attack against the world **as the shooter saw it**, within a limit. It is a library: nothing in a server
calls it yet (see the end).

## The problem and the idea

A shooter's screen shows the other players a little in the past: the network delay plus the client's interpolation delay. When the shot reaches the server the target has moved on, so a shot that was aimed
exactly on the screen misses on the server, and a player with a slow connection never hits a moving target. The fix is to **rewind the targets** for the duration of the shot:

1. Every simulated tick the server records the moving colliders (`LagCompensator::record(tick, colliders)`): one entry per tick, built as a `QueryWorld` with no static geometry, so a query is the ordinary one.
2. A shot arrives with the tick of the snapshot the shooter was showing (`view_tick`; the delays are already inside it). `rewind(current_tick, view_tick)` says which tick to use and why.
3. `raycast(live, &rewind, ray, max_distance, filter)` asks the **live static world** (`live`, queried for statics only) and the **historical moving colliders** and returns the nearest hit.
   A rewound target is therefore never hit through a wall that exists now, and the live positions of the moving colliders are never consulted. `world_at(tick)` gives the historical world for casts and
   overlaps (a melee swing).

## Fairness is a limit

The rewind is bounded, because a rewind is a way to act on old information, and a player who fakes lag could otherwise reach arbitrarily far back (`docs/THREAT_MODEL_NET.md`, "lag switch"):

| `RewindOutcome` | When | Tick used |
|---|---|---|
| `Exact` | the view is within the window (including 0 ticks back) | the view tick |
| `Clamped` | the view is older than `max_rewind_ticks` | the oldest allowed tick |
| `FutureIgnored` | the view tick is ahead of the server's | the current tick (a client cannot ask to be ahead) |
| `HistoryShort` | the history does not reach back that far (a new match, a gap, nothing recorded) | the oldest entry, or the current tick if there is none, never later than the current tick |

`Rewind` carries `requested_ticks` and `applied_ticks`, so **the server can count a client that keeps asking for more than it may have** (that is the input to a kick or warn policy; the policy is the game's). The window
is configured in ticks (`LagCompConfig::new(n)`, 0 turns the feature off) and cannot exceed `MAX_REWIND_TICKS` = 256, so the memory is bounded whatever a configuration says; the history never holds more than
`window + 1` entries however long the game runs (a test records 10,000 ticks).

What the window should be is a **game-design decision**: a shooter whose lag exceeds it has to lead the target. The tick rate (T01-022) is not known yet, so the numbers are ticks, not milliseconds; at 60 Hz a window
of 12 ticks is 200 ms. I have not chosen a default and propose none.

## Details that matter

- **Gaps are allowed**: `record` needs only a newer tick than the last, and `world_at(t)` is the last entry recorded at or before `t` (`None` if `t` is older than the history). A tick that is not newer is refused and changes nothing.
- A moving collider at the same distance as the static world wins, as in `QueryWorld`.
- Colliders may be given in any order (the world sorts them by id), so the answer does not depend on it.
- Pure and deterministic: integer ticks, a `VecDeque`, no clocks, no hash maps (the crate is sim-safe).

## How it was tested

Nineteen tests and a doctest, written first and seen to fail on the missing API: the five rewind outcomes and their boundaries (exactly at the window is `Exact`, one tick more is `Clamped`; a zero window; a short history; a
current tick older than the oldest entry never moves forward; no overflow at `u32::MAX` or below tick 0); recording (only the window is kept over 10,000 ticks; a tick that is not newer is refused; the window cap; gaps); and the shots: a shot at where the
target was hits only with the rewind and misses at the current tick, a shot cannot reach past the window, a live wall in front of a rewound target blocks it and one behind it does not, the shooter's own body and the layer mask are
respected, the live world's moving colliders are not consulted, the rewound world answers an overlap, and the order colliders are recorded in changes nothing. **Thirteen deliberate breakages each fail a test** (the future boundary, no clamp, the clamp flag,
the oldest-entry boundary, an equal tick accepted, trimming off by one, the cap off by one, the `world_at` boundary, the static world preferred, no wall limit, the live moving colliders consulted, a forward rewind, `applied_ticks`
reporting the request). The first geometry I wrote was wrong (a target moving one unit per tick behind a nearly flat ray is hit from the current position as well), found by a failing test and fixed by making the target faster.

## Not covered (follow-up T08-010b)

- **Nothing calls it.** The server must record the colliders of every tick, take the view tick from the shooter's input (the field does not exist: `InputBuffer` carries opaque payloads), call `rewind` and count the outcomes.
  The entity model (T08-002) and the server and client split (T08-004) come first, and the **view tick** has to be derived by the client from its interpolation clock (`TickClock`, T08-016) and checked by the server.
- **Interpolation between ticks.** A client draws a position between two snapshots; this module uses whole ticks, so a shot can be off by up to half a tick of movement. Blending two entries by collider id is the usual answer and is not built.
- **Hitboxes that are not the movement collider** (head, limbs: T07-010) and rewinding **animation poses**; the history holds the shapes it is given.
- **Capsule and box casts**, until T07-004b exists; a melee swing is only an overlap today.
- **Cost.** Recording builds one BVH per tick (the live world builds one per tick already, but this is a second one). I have not measured it with a full match (T07-017 has the benchmark harness); the memory is bounded, the time is not measured.
- **Favour-the-shooter side effects** (a target shot "after" reaching cover), which the game may want to limit with a smaller window or a rule; not modelled.
