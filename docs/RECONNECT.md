# Reconnecting and dropping in and out

`ss2_net::reconnect` (T08-014) answers two questions the session layer (`docs/SESSION.md`) leaves open: **who keeps a seat when a connection drops**, and **how a client tries again**. It is pure logic like the rest of `ss2-net`: time is a number of microseconds passed in, there is no randomness (tokens are supplied by the caller) and there is no `HashMap`. It is **not wired into the session messages yet** (see the end).

## The server: `SeatKeeper`

A player is given an unguessable **token** when they join (`player_joined(slot, name, token)`; the caller makes the token). When their connection ends, `player_left(slot, reason, now, last_acked_tick)` returns an `Outcome`:

- **`Held`**: the reason was `TimedOut`, so the seat is held for the grace period (60 s by default) under that token. The held seat is **not given to a new player** (`free_slot(max_players)` skips it), and the server remembers the last tick the player is known to have received.
- **`Released`**: the player quit, was kicked or banned, or the server stopped (leaving on purpose forfeits the seat), or there is no room to hold it (at most 8 held seats by default, so a flood of dropped connections cannot hold the whole server).
- **`NotSeated`**: nobody was seated in that slot.

A returning player calls `claim(token, name, now)` and gets a `Claim`:

- **`Resume { slot, last_acked_tick }`**: the same slot as before. The caller decides between catching up from that tick and sending a full snapshot (join-in-progress, T08-013).
- **`Unknown`**: no seat is held for that token (never issued, expired, already taken back): treat the player as new.
- **`WrongName`**: the token is right but the name differs (names compare without case).
- **`LockedOut`**: ten wrong tokens or wrong names have been tried since the last time no seats were held: claims are refused, even the right ones, until `reset_lockout()` or until the last hold ends. A guess made while **no** seat is held is not counted (there was nothing to guess at).

The token is compared against **every** held seat in constant time (`session::constant_time_eq`, no early exit), so the time taken does not say how close a guess was. A hold that has run out is not honoured even if `expire(now)` has not run yet; `expire` (call it every tick) returns the slots whose holds ended, in order, so they can be freed. A token handed out twice (a bug of the caller) is resolved safely: the newest owner wins, any reservation under that token is dropped, and an earlier connected player with it loses their seat entry.

## The client: `Backoff`

`Backoff::new(policy, dropped_at)` schedules the retries of a dropped connection; `poll(now)` says `Now`, `WaitUntil(t)` or `GiveUp`.

- The waits **double from 0.5 s up to 8 s** (0.5, 1, 2, 4, 8, 8, ...), at most 12 attempts, and it gives up 90 s after the drop (a little longer than the 60 s seat hold).
- **`refused(now, reason)` splits the refusals.** Permanent ones **stop** it: a ban, a version or schema or mod mismatch, a wrong password, a failed auth, an invalid name (none of these will change by waiting). Transient ones go on: the server is full, the name is still held by the old connection, or a rate limit, whose `retry_after` is honoured if it is longer than the schedule.
- `connected(now)` starts the schedule over for the next drop (the attempts, the give-up clock and a stopped state are all reset).

The shifts and sums saturate: tested with 200 failures at the largest time values.

## How it was tested

Twenty-three unit tests and two doctests: holding and resuming (and dropping again), every leave reason, held seats blocking free slots, expiry at the exact microsecond (both in `expire` and in `claim`), the held-seat limit, wrong names and tokens and the lockout (including that it survives while any seat is held and clears with the last), a token one byte off, a repeated token, the doubling waits, the attempt and time limits, permanent against transient refusals, a rate limit that asks for more or less than the schedule, the reset, overflow, and a whole drop and return through both sides. **Twenty-four deliberate breakages: sixteen failed a test at once and seven survived my first tests.** One survivor is equivalent: comparing tokens with `==` instead of in constant time gives the same answers (the point is the timing, which a unit test cannot see; the constant-time function itself is tested in `session`). The other six were real gaps (when the lockout clears, two cases of a repeated token, a reconnect resetting only the attempt count, and a poll that is not due exactly at its time) and each failed a test once I added one; the last of them makes the combined drop-and-return test loop forever, so it is caught by a timeout rather than an assertion. The tests also made me replace a first version of `player_left` that returned `None` for both "held" and "nobody seated" with the `Outcome` enum, and one mutation (a new one, after that change) was killed at once.

## Not covered (follow-up T08-014b)

- **Not in the protocol.** The session messages do not carry a token: `Welcome` would have to give one and `Hello` would have to present it, which changes the protocol version (`docs/PROTOCOL.md`). Until then `SessionServer` still forgets a timed-out player, and nothing calls `SeatKeeper` or `Backoff`.
- **No state is restored.** `Resume` says which slot and tick; sending the player their state again (a full snapshot, their inventory and health) is join-in-progress (T08-013) and the game layer. How long a dead connection's *player entity* stays in the world (a ghost, a frozen body, removed) is a gameplay decision this does not make.
- **Host migration (the optional part of the title) is not done**: it needs the transport and a way to hand the authoritative simulation to another machine.
- **No network-change handling** (Wi-Fi to cellular, T08-031) and no jitter in the retry delays (several clients dropped by one outage would retry in step; a seeded jitter is a small addition for the follow-up). The 60 s grace, the 8 held seats and the delays are my numbers.
