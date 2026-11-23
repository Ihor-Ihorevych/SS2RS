# Save files

`ss2_sim::save` (T08-022) writes the entities of a world as bytes and reads them back, in **a format of our own** (nothing from the original's saves). It works on any
`World` (`docs/rfcs/0009-sim-contract.md`): `save(&world, &header)` gives the bytes, `load(&bytes, &mut empty_world, &limits)` builds them into an empty world, and `parse` / `restore` do
the two halves separately.

## What is saved

- the **persistent properties** (`PropertyDesc::persistent`) of every entity, and its class and **parent**;
- a header: the **tick**, the random **seed**, the number of **players** (a co-op save says how many) and up to 16 **notes** (the level, the difficulty, a title for the save list; each key and value at most 256 bytes).

What is **not** saved comes back as its default: properties that are not marked persistent, the world's change and lifecycle records, the event queue, and **anything the game's behaviour keeps outside the
properties of an entity**. That last point is a rule for the code that is not written yet: **a system's private state must live in properties to survive a save**.

## The checks

[`parse`] reads the file and builds nothing until every check has passed: the magic, the **format version**, a **CRC-32 of every byte** (one flipped bit anywhere is caught: a test flips each byte of a file in turn),
the **schema hash** of the class registry (`ClassRegistry::schema_hash`: a save made with other classes or other property layouts is refused, not misread), every class id, every value (a boolean that is neither 0 nor 1, text that is not UTF-8),
entities in strictly ascending slot order, **parents that must be saved entities with the saved generation** (a stale handle does not become whatever entity now uses the slot), **no parent cycles**, and no bytes after the last entity.
**Limits** stop a damaged or hostile file from making the reader allocate: entities (default about a million), the longest text (64 KiB), the file (256 MiB), and an entity count that the file is too short to hold is refused
before anything is reserved (a test claims a billion entities in a tiny file).

## Loading

`restore` needs an **empty** world (otherwise `WorldNotEmpty` and nothing changes) whose registry has the save's schema hash. It spawns the entities in saved order, parents them, then sets their values.
**Entities get new handles**; the saved handles are only used to rebuild references: an entity-valued property is mapped to the new handle, and one that pointed at an entity that was not saved (or at a stale handle) comes back as the
**null entity and is counted** in `Restored::dangling_refs`. The world's lifecycle records hold the spawns afterwards; drain them if they are not wanted. A world that loads and is saved again gives the same bytes as the next load
(handles are compacted from slot 0), and **two worlds built by the same calls give the same bytes** (entities are written by slot index; floats are stored as their exact bits, so `-0.0` and NaN survive).

## The layout

All integers little endian: `"SS2S"`, format version `u16` (1), schema hash `u64`, tick `u64`, seed `u64`, players `u8`, notes (`u8` count, then pairs of `u32` length and UTF-8), entity count `u32`, then for each entity its index `u32`,
generation `u32`, class `u16`, parent `u64` (all ones for none) and its persistent property values in slot order (bool 1 byte, `i32`/`u32`/`f32` 4, vector 12, text `u32` length and UTF-8, entity `u64`), and the CRC-32 of everything before it.
There is no compression, no encryption and no thumbnail.

## What a save is worth across builds

**Every change to the classes (a property added, renamed, retyped, reordered, its replication changed) changes the schema hash, and an older save is then refused.** There is no migration. That is deliberate for now (a
refused save is better than a misread one), and it means saves are good only for the build that made them until a migration story exists. The hash also covers wire details a save does not need (replication mode, quantization),
so it can be stricter than necessary.

## How it was tested

Nineteen tests, a fuzz target and mutation checks. The tests: a world with gaps, a hierarchy, references, Unicode text, NaN, infinity and `-0.0` survives a round trip (every persistent property bit for bit, every parent, every reference
remapped; unsaved properties come back as defaults); saving is deterministic and loading compacts handles; stale and null references; an empty world; 5,000 entities; **every prefix of a file is refused, and every byte flipped is refused**;
another version; another schema (in `parse` and in `restore`); each limit at its boundary; checksum-correct damage (an unknown class, a parent that does not exist, a repeated or null index, trailing bytes, text that is not UTF-8, a boolean of 7);
hand-made files for cycles, stale generations and unordered entities; a world that is not empty; headers that cannot be written. **Seventeen deliberate breakages each fail a test** (checksum ignored, schema ignored in either place, the entity and
file and text limits off by one, no check of the count against the file size, order, null and stale-generation checks removed, a non-empty world accepted, dangling references not counted, the checksum over the wrong bytes, notes not written, the notes limit
off by one, the CRC polynomial). Five of them survived my first tests (order, the text limit, a stale parent generation, and two that were really one redundant check, which I removed), so I added the hand-made-file and text-limit tests.
The fuzz target `save_parse` (see `docs/FUZZING.md`) reads raw bytes and a scripted world that is saved, damaged and **re-sealed with a correct checksum** (otherwise the checksum hides every other check), and checks that what reads
restores, saves and reads again. I ran it for **60 seconds, 30,627 runs, no finding**; that is a smoke test, not coverage, and it has not run in CI.

## Not covered (follow-up T08-022b)

- **Checkpoints and quick-save rules** (when saving is allowed, how many slots, what happens in co-op when one player quick-saves): game-design decisions, and the roadmap line asks for them; none is built.
- **Writing the file safely** (a temporary file, then a rename; a backup of the last good save; where saves live: `ss2-platform-api` has the locations) and a **save list** (reading just the header of every file for the menu; `parse` reads the whole file).
- **Co-op transfer**: sending a host's save to joining clients (it is the join-in-progress snapshot, T08-013, that carries a running game; a save is for resuming later), and which player slot is whose after a load.
- **Migration** between builds (above), **compression**, **thumbnails**, and **save games of the original** (not planned; they are not our format).
- **Speed and size** are not measured: a 5,000-entity round trip is inside a test that runs in well under a second, and that is all I know.
- **The real world implementation** (T08-002): the module is written against the `World` trait and tested only with `SimpleWorld`.
