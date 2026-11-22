# Cheat and developer commands

The console (`ss2_core::console`) gets a set of cheat commands from `ss2_game::cheats` (T09-028). They are for testing
and for playing around; they are **off by default**.

## Turning them on

Every command refuses to run until the console variable `sv_cheats` is `1`:

```
sv_cheats 1
```

With `sv_cheats 0` (the default) a cheat command answers `cheats are disabled (set sv_cheats 1 to allow them)` and does
nothing. The name starts with `sv_` because in a co-op game the **host** decides; see "Co-op" below.

## The commands

| Command | Effect | Notes |
|---|---|---|
| `god [on\|off]` | the player takes no damage | no value: toggles |
| `noclip [on\|off]` | the player flies through walls | no value: toggles |
| `notarget [on\|off]` | enemies do not notice the player | no value: toggles |
| `give <item> [amount]` | adds an item to the inventory | amount 1 to 9,999 (default 1); `give all` is the usual "everything" request |
| `spawn <class> [count]` | creates enemies or objects in front of the player | count 1 to 64 (default 1) |
| `killall` | kills every enemy of the level | |
| `skip` | finishes the level at once | |

Names are lower-cased and may contain letters, digits, `_` and `.`; anything else is refused. The console says what it
understood (`give shotgun x3`). Whether the item or class exists is decided when the game applies the request, because the
cheat layer does not know the game's data.

## How it works (for developers)

The commands **change nothing themselves**. Each valid command appends a `CheatRequest` to a queue and the sticky switches
are kept in `CheatFlags`:

```rust
let mut console = Console::new();
let cheats = Cheats::register(&mut console)?;       // declares sv_cheats and the commands
console.execute("sv_cheats 1")?;
console.execute("give shotgun 3")?;
for request in cheats.take_requests() {              // once per frame, in the game loop
    apply(request);                                  // CheatRequest::Give { item: "shotgun", amount: 3 }
}
```

`take_requests` drains the queue oldest first; `flags()` reads the switches. A refused command (cheats off, a bad value,
the wrong number of arguments) queues nothing.

## Co-op

A client must not change game state on its own. The game must turn each `CheatRequest` into a **simulation command**
(T08) that the host validates (and that is refused when the host has `sv_cheats 0`), so that the host and all clients apply
the same change on the same tick. This is not wired yet: there is no console window (T10) and no simulation to apply the
requests to (T08, T09 player and entities). Until then the layer is tested with a plain `Console`.

## Not covered

- Level selection by name (`map <name>`): needs the level list (T02, T09-027); `skip` goes to the next level only.
- Saving a cheat state, or marking saves and scores as made with cheats.
- Cheats in the menu, and key bindings for them (the action map of T03-005 can bind console lines later).
