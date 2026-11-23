# Spectating when dead

`ss2_game::spectate` (T08-024) is what a dead co-op player sees: **whom to watch** (`Spectator`) and **the camera that trails them** (`FollowCamera`). It is pure logic: no window, no network and no clock (time is the `dt` argument), so it is tested without a GPU. It builds on the free-fly camera of `ss2_game::freefly`, which is the fallback when nobody is left to watch.

```rust
let mut spectator = Spectator::new(local_player_id);
spectator.begin(&roster);                 // I died
// every frame:
spectator.update(&roster);                // moves on by itself if the watched player dies or leaves
if let Some(id) = spectator.target() { follow_cam.update(dt, position_of(id), yaw_of(id)); camera = follow_cam.camera(); }
else { /* free-fly camera */ }
```

`roster` is a list of `Candidate { id, alive }` (any stable player ids; their order is the cycling order).

## Choosing a player

- Only **living players other than yourself** can be watched.
- `begin` (the local player died) picks **the player watched last time if they are still alive**, otherwise the first living one by id; with nobody alive it flies freely.
- `update` keeps the choice valid: if the watched player **dies or leaves the roster**, it moves to the **next living player after them in id order, wrapping** (not back to the start of the list); if nobody is alive it falls back to **free-fly (`by_choice: false`)**, and it returns to watching by itself when someone is alive again (the last watched player if they are, otherwise the first living).
- `next` and `previous` cycle through the living in id order and wrap; from free-fly, `next` continues after the last watched player, and with no history `next` starts at the first and `previous` at the last.
- `toggle_free_fly` is the player's choice of the free camera (`by_choice: true`): **it is not overridden** by who is alive, and toggling back returns to the last watched player (or the first living one if they are gone). With nobody alive it stays free.
- `end` (respawned) stops. A spectator that is `Off` ignores every other call.

## The follow camera

`FollowCamera` sits `distance` (3) behind and `height` (1.8) above the watched player, looking at a point `look_height` (1.5) above their position, **smoothed** with an exponential approach (`smoothing` = the time to close about two thirds of the gap; 0 snaps). Details that matter:

- The first update, and the first after `snap()` (the watched player changed or teleported), **place the camera at once**: it does not fly in from the origin.
- A step is limited to 0.25 s, so a stall does not throw the camera across the level; a negative, NaN or infinite `dt`, or a target that is not finite, **changes nothing**.
- With no smoothing a zero-length step still places the camera (the smoothing formula would otherwise divide 0 by 0 and give NaN; a test covers it).
- The direction convention is the free-fly camera's: yaw 0 faces -Z and a positive yaw turns left, so a player facing -X (yaw a quarter turn) has the camera on the +X side. The camera looks straight down without flipping if the height is large and the distance 0.

## How it was tested

Nineteen unit tests and two doctests: starting on the first living teammate and never on yourself, the watched player's death moving on to the *next* id (and wrapping), leaving the roster, free-fly and the return from it, cycling both ways, stepping with nobody alive, free-fly by choice and the memory of the last player, duplicate and unsorted rosters, the neighbour helpers at their edges, the camera's position for four facings, looking at the head, the smoothing's monotonic approach and the step cap, the first-update snap, bad inputs, and the projection. **Twenty-four deliberate breakages: nineteen failed a test at once, one was rejected by the compiler (a non-exhaustive match: not counted as a test catch), three survived, and one did not apply the first time (see below):** one equivalent (sorting kept, the de-duplication removed: the strict comparisons used to find the next player make a duplicate in the list harmless, so I **removed that code**) and two real (what `toggle_free_fly` remembers; and the zero-smoothing case above, which turned out to be a **latent NaN**: with smoothing 0 and a zero-length step the camera position would have become NaN). The one that did not apply (putting the camera in front of the player) was a line rustfmt had reflowed; redone, it failed a test.

## Not covered (follow-up T08-024b)

- **No game around it**: nothing makes the local player a spectator when they die, nothing supplies the roster, positions or yaws of the other players (that is the replicated player state of T08-007 and T08-009), and no key or button calls `next`, `previous` or `toggle_free_fly`. Rules for **who may watch whom** (teams, only teammates in a competitive mode) are not modelled: the roster the caller passes in is the rule.
- **The camera does not collide**: it can end up inside a wall behind the watched player (a collision-aware pull-in needs the world and `ss2-physics` queries), and it does not follow the player's pitch or offer a mouse-orbit.
- **No HUD**: the name of the watched player, the "press to switch" prompt and a death screen are the UI's (T10-020).
- **The numbers are mine**: distance, height, look height and smoothing are plausible defaults, not matched to the original.
