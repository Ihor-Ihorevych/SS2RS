---
name: Desync (co-op peers disagree)
about: Two or more co-op peers see different game state (positions, health, enemies, doors)
labels: ["bug", "desync"]
---

<!-- Do not attach game files or copied game text. Replay files are attached only after checking that they contain no game content. See docs/TRIAGE.md section 8. -->

**What diverged** (what the players saw differ: a position, health, an enemy, a door, a pickup):

**Time of first divergence** (the tick from the log if you have it, otherwise roughly how many minutes into the session):

**Steps to reproduce** (level, number of players, the inputs or actions that led there):

**Frequency:** every session / sometimes / once

**Session**
- Topology: LAN or internet; host and number of clients:
- Did the same two peers desync on a second run? (yes / no / not tried):
- Level or map name:

**Seed and replay**
- Seed (from the session log, or "not printed"):
- Replay file, if one was recorded: attach it only after checking that it holds no game content; otherwise say that it exists and do not attach it.

**Expected:**

**Actual:**

**Environment** (copy this block for every peer)
- Peer name or number (host / client 1 / ...):
- Version or commit (must be the same on all peers; say so if not):
- OS and version:
- GPU and driver version:
- `SS2_DATA_DIR` points to an unmodified retail install? (yes / no / not sure):
- Game store and version and patch level (names of the `Patch_*` files are enough; all peers should match):

**Logs:** attach the log of **every peer**, with `SS2_LOG=debug`, from the same session. Remove user names and personal paths. Say which log belongs to which peer.
