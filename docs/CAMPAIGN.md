# Campaign flow: the level list, level transitions, carry-over, difficulty carry, unlocks and the save note

`ss2_game::campaign` (T09-025) is the **model of how a campaign moves from level to level**: an ordered list of levels with their movies and exits, the stages of a transition (outro movie, load, episode movie, intro movie, spawn), what the players carry from one level into the next, the difficulty that goes with them, what finishing levels unlocks, and the note a save keeps. It is pure logic: time is a count of ticks, there is no clock, no randomness and no `HashMap`, and the same events always give the same state, byte for byte. **Nothing in the game calls it yet** (see "Not covered"): it has been exercised by its own tests and by two data-gated tests that build and walk the level list of the real install.

The code is `crates/ss2-game/src/campaign/`: `list.rs` (the level list), `gate.rs` (the co-op exit rule), `carry.rs` (carry-over), `unlocks.rs` (unlock rules and set), `state.rs` (the state machine and the save note), `codec.rs` (the shared byte envelope).

## What the data says, and what I chose

The reverse-engineering notes say little about the flow between levels, so most of the rules are **mine**. Each rule below has a key (`ss2_game::campaign::Rule`), is marked **Spec** (written down in `docs/re/`) or **Chosen** (not in the data), and a test checks that this table and `Rule::ALL` agree.

| Rule | Provenance | What it says |
|---|---|---|
| `level-set` | Spec | The campaign is the 42 levels whose `.nfo` says `MAP_TYPE = SINGLE; COOP;` (`game-modes.md`); the 16 deathmatch levels are not part of it. |
| `level-order` | Spec | The levels are numbered `N_M` (`1_1_jungle` to `7_7_mental`, seven episodes of 8, 6, 5, 4, 7, 5 and 7 levels); the order is the numbering. The data has **no** next-level field and says that what loads the next level is not stated (`level-logic.md`), so "the number after" is how I read it (`game-modes.md`, `cutscenes.md`). |
| `movies-per-level` | Spec | A level's `.nfo` may name `INTRO_MOVIE`, `OUTRO_MOVIE` and (the first level of an episode) `EPISODE_MOVIE`; any may be missing or empty (`cutscenes.md`, `misc-types.md`). |
| `outro-then-load` | Spec | A chapter with a next level ends the level; the outro movie of the `.nfo` plays and the next level loads (`help-digest.md`, `cutscenes.md`). |
| `start-inventory` | Spec | A chapter holds the player's start inventory: weapons, ammo, health, armor, preferred weapon (`player-rules.md`, `help-digest.md`). Nothing says that anything is carried between levels. |
| `list-shape` | Chosen | A level id is 1 to 64 characters of letters, digits, `_`, `-` and `.`; a list has 1 to 1,024 levels with unique ids; names and movie references are at most 256 bytes without control characters, and a movie reference is never empty. |
| `list-validation` | Chosen | A list is refused (typed `ListError`, checked in this order: shape, duplicate ids, dangling links, cycles, unreachable levels) when a link names a level that is not listed, the last level "follows" nothing, the exits make a cycle (found without recursion), or a level cannot be reached from the first. |
| `exits` | Chosen | A level has a **main exit** (exit 0: the following entry, a named level, or the end) and up to seven **alternative exits** to named levels (secret levels). The first entry is where the campaign starts. A secret exit is the only branching the model has; the data shows none (every retail level leads to the next number). |
| `level-kinds` | Chosen | A level is `Main`, `Secret` or `Bonus`. Only a `Main` level that is left through an exit that ends the campaign **completes** it. |
| `stage-order` | Chosen | A transition has the stages **Outro, Load, EpisodeMovie, Intro, Spawn**, each ended by an event from the game; a stage with no movie is skipped. The order of the episode movie and the intro, and that the intro comes after the load, are mine (`cutscenes.md` does not say). |
| `no-movies-on-restart` | Chosen | A restart of a level, and a campaign loaded from a note, go through Load and Spawn only: no outro, episode movie or intro plays. |
| `when-to-leave` | Chosen | A transition may start only while a level is being played, for that level, through an exit it has, when the exit gate says go, when the result is possible (no more secrets found than exist), and when every living player has carried state in the request. A refused call changes nothing. |
| `gate-everyone` | Chosen | The exit gate waits for every player who is connected and alive to be at the exit; dead players do not block it, disconnected ones do not count. |
| `gate-timeout` | Chosen | The first arrival starts a timer of `timeout_ticks`; when it runs out the level is left without those still away. The timer only runs while somebody is at the exit and restarts from zero when nobody is. A timeout of 0 goes as soon as one player is there. |
| `ghost-keeps-state` | Chosen | A living player who is not at the exit when the timeout ends is **ghosted**: they are carried to the next level with exactly the state they had, and the list of them is handed to the game. (What a ghost may do in the new level is the game's business.) |
| `nobody-alive` | Chosen | The gate never opens when nobody who is connected is alive; the level is then restarted (the caller decides when). |
| `difficulty-carries` | Chosen | The difficulty is unchanged by every transition and by a restart; it may be changed between levels under its `ChangePolicy` (`docs/DIFFICULTY.md`): not while a level is played under the default policy, never under `Locked`, always under `Anytime`. |
| `carry-floor` | Chosen | A living player carries health, armor, weapons, ammo and items; the next level raises each to the chapter's start inventory where it is lower (the larger of the two for health, armor and every key; the preferred weapon stays theirs). With `CarryPolicy::Reset` nobody carries anything. |
| `dead-and-late-start` | Chosen | A player who was dead when the level was left, a player who joins later and a slot with no entry start the next level with the chapter's start inventory alone; a disconnected player's entry is dropped. The gate, not the request, decides who is dead. |
| `restart-keeps-entry` | Chosen | A restart begins again from what the level was **entered** with (not from what the players had when they died), counts the restart, and keeps the elapsed time. (The chapter inside the level is `level_progress`'s business.) |
| `elapsed-only-in-play` | Chosen | The campaign's elapsed ticks only count while a level is being played (not during a transition), saturating. |
| `unlock-exit-target` | Chosen | Finishing a level unlocks the level its exit leads to; the first level is unlocked at the start. |
| `unlock-triggers` | Chosen | A table of rules supplied by the caller may unlock levels, cheats and modes when a level is finished, when at least N of its secrets were found, when all of them were (never for a level without secrets) and when the campaign is complete. At most 256 rules of 16 grants; at most 4,096 things unlocked. |
| `save-only-when-playing` | Chosen | A note is only written while a level is being played or when the campaign is finished; during a transition there is nothing consistent to save. |
| `fingerprint` | Chosen | A note carries a fingerprint of the level list (CRC-32 of the ids, their order and where each exit leads) and is refused with another list. |

## The model

- **`LevelList::new(entries)`** validates and builds the list (`LevelEntry`: id, episode, display `name`, `LevelKind`, `Next` for the main exit, `alt_exits`, the three movie references). `destination(id, exit)` says where an exit leads (`Destination::Level` or `End`), `main_path()` follows exit 0 from the first level, `fingerprint()` identifies the list.
- **`ExitGate`** is the co-op rule: each slot is `Away`, `AtExit`, `Dead` or `Gone`; `advance(ticks)` runs the timer; `decision()` is `Wait { remaining }`, `Go { ghosted, dead }` or `NobodyAlive`.
- **`CarryOver`** is a map from player slot (0 to 63) to `Slot::Carried(PlayerCarry)` or `Slot::Dead`. `PlayerCarry` is health, armor, a preferred weapon and three plain `BTreeMap<String, u32>` (weapons, ammo, items); keys are 1 to 64 bytes, at most 256 per map, no zero counts, the preferred weapon must be owned. `resolve(slot, chapter_start, policy)` is what a player starts the next level with. The byte format is `"S2CO"`, version 1, little endian, slots and keys in ascending order, CRC-32.
- **`Campaign`** is the state machine. `Campaign::new(list, difficulty)` is a new game in the Load stage of the first level. `leave(LeaveRequest { gate, carry, result, rules })` starts a transition (or the end of the campaign) and returns a `Departure` (where to, who was ghosted, who was dead, what was newly unlocked, whether the campaign is complete); `progress(Progress::MovieDone | Loaded | Spawned)` ends a stage; `restart()` starts the level again; `advance(ticks)` counts time; `request_difficulty(level)` goes to `CampaignDifficulty`; `current_movie()` says which movie to play now; `start_inventory(slot, chapter_start, policy)` says what a player starts with. The phases are `Playing`, `Transition { from, to, stage, movies }`, `Ending` (the last outro) and `Finished`.
- **`unlocks_after(rules, list, result)`** is the pure unlock function: the level the exit leads to first, then the grants of every rule that fires, in table order, without repeats. `LevelResult::from_summary` takes the counters from `level_progress::LevelSummary`. `UnlockSet` is the persisted set (`"S2UL"`, version 1, sorted).
- **`CampaignNote`** is what a save keeps: the list fingerprint, the level id, the difficulty (its four bytes from `CampaignDifficulty::to_bytes`), the carry-over the level was entered with, the unlock set, the elapsed ticks, the restart count and a finished flag (`"S2CN"`, version 1). It is a byte block, not a text note: it does not fit the 256-byte notes of the save header (`SAVES.md`), so embedding it in a save is a follow-up. `Campaign::restore(list, &note)` / `restore_bytes(list, bytes)` bring a campaign back in the Load stage of the saved level with no movies.

Every `from_bytes` refuses with a typed `BytesError` (too short, bad checksum, wrong magic, unknown version, truncated, trailing bytes, or `Invalid(why)`); a different version is refused, there is no migration.

## The real install

`crates/ss2-game/src/campaign/data_tests.rs` (runs with `SS2_DATA_DIR`, skips without) mounts the install with `ss2_vfs::mount_install`, reads every `.nfo` under `content/serioussam2/levels` with `ss2_vfs::Nfo`, keeps the `LEVEL` files whose `MAP_TYPE` has `SINGLE`, takes the level id from the folder name (the `.nfo` stem differs in places), the episode from the folder above, orders them by the `N_M` numbering and builds a `LevelList`. **It is the reference for how a caller builds the list; the module itself does not read files** (`ss2-game` has no run-time dependency on `ss2-vfs`).

RESULTS_PLACEHOLDER

## How it was tested

TESTS_PLACEHOLDER

## Not covered

- **Nothing calls it.** No level is loaded, no movie is played, no player entity exists to give or take a `PlayerCarry`, no exit trigger drives the `ExitGate`, no menu asks for a difficulty or shows the unlocked levels. The module only answers questions and keeps state. (Follow-up T09-025b.)
- **No real next-level link.** The data does not state what loads the next level (`level-logic.md`: the user event `StartNextLevelLink` and `CPortalEntity` are the likely mechanism, not read). The list's order is the numbering of the ids; reading the chapters' "next level" property from the `.wld` files and checking it against that order is not done. There is **no branching in the retail data that I know of**: secret exits, secret and bonus levels are supported by the model but the real list has none.
- **No world, no inventory contents.** `PlayerCarry` is a plain shape; this crate has no player, weapon or ammo types, no caps on counts (the player entity owns them), and no real chapter start inventories (they are in the `.wld` chapter entities, not read).
- **No save file.** The note is a byte block; it is not embedded in `ss2_sim::save` (the header's notes are limited to 256 bytes) and there is no slot or quick-save rule. The level-progress state (`level_progress`) and the world snapshot are separate and not part of the note.
- **No unlock rules from the data.** There are none: the retail data has an unlock-key screen (`SMSEnterUnlockKey`) and a cheats screen that lists chapters, and no rule for what unlocks what. The unlocks here are my own and the table is supplied by the caller; the five `medallion_N` clips are not used. Cheats and modes unlocked are just ids: nothing connects them to `cheats.rs` or a mode manager (T09-002).
- **The movies.** The module says *which* movie to play and when; it does not open, decode or skip one. Whether the player may skip a movie is not in the data (`cutscenes.md`); the event is `MovieDone` either way.
- **Co-op specifics.** Who may change the difficulty in a co-op game, how a ghost behaves, whether a dead player's score or kills are kept, what happens to a disconnected player who returns, and the lobby's role are open. A player who disconnects is dropped from the carry-over and starts from the chapter if they come back.
- **Campaign-wide counters.** Only the unlock triggers look at one level's secrets; there is no total of secrets, kills or time across the campaign (the elapsed ticks are the only total), no score, no extra lives (the 10,000-point rule is not here).
- **Linux/WSL runs**: see the pull request.
