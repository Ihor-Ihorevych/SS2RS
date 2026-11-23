# Co-op overlays

`ss2_ui::coop_overlays` (T10-014) holds the five things a co-op game draws over the world: the **scoreboard**, the **chat**, **nameplates** over teammates' heads, **ping markers** and **voice indicators**. Like the other screens they know nothing of the game: they take plain data (players, typed text, positions the caller has already projected to the screen) and **append** quads and text runs to a `DrawList` in canvas units (1920x1080) with a placeholder look, so that several overlays and the HUD can share one list (the caller clears it once a frame). `CoopOverlays` holds all five and draws them in one order: nameplates, pings, voice list, chat, scoreboard on top.

**Nothing feeds them yet**: no game, no network and no input mapping calls them (see "Not covered").

## No font metrics

`ss2-ui` does not know how wide a string is drawn. Text is placed by anchor (left or centred) in fixed columns, lengths are limited in **characters**, and where the layout needs a width (keeping a ping label on the canvas) it uses `estimate_text_width` (0.55 x the font size per character, a guess on the wide side). `clip_text` removes control characters, trims, and cuts with an ellipsis. **Strings that vary** (names, chat rows, scores, distances) are owned `String`s, so drawing allocates a short string per number or name and each nameplate/ping/scoreboard draw builds a small `Vec`; this is **not allocation-free** like the menu of `docs/UI_BUDGET.md`.

## Scoreboard

Hold a key (`set_held`) and `set_entries` rows of `ScoreEntry` (id, name, kills, deaths, score, ping, local, talking).

- **Sorted** by score (high first), then kills (high first), then deaths (low first), then id, so the order is the same on every machine whatever order the players arrive in.
- Names are cleaned and cut to 20 characters; an empty one becomes "Player N".
- **At most 16 rows**; further players are counted in a "+N more" line, and **the local player is always shown**: beyond the sixteenth place it takes the last row.
- The local row is orange; a talking player has a green dot; the ping figure is green under 80 ms, yellow under 150 ms, red above, capped at 999, and "-" when unknown.

## Chat

`ChatBox` keeps a history and an input line.

- **History**: `push(kind, sender, text)` with `Player` ("Name: text"), `Team` ("[Team] Name: text") and `System` (the text alone). Text is cleaned and cut to 200 characters, a line with no text is ignored, lines are wrapped at 56 characters (at a space when there is one, else inside the word) and **at most 100 lines are kept** (the oldest go), so it cannot grow without limit. At most 8 rows are drawn, newest at the bottom.
- **Fade**: `update()` once per tick; a line stays fully visible for 600 ticks, then fades over 60, then is not drawn. While the input is open every kept line is shown at full strength.
- **Input**: `open(team)`, `handle_text` (control characters left out, at most 200 characters), `handle_key` with Enter, Escape, Backspace, Delete, Left, Right, Home, End and Tab (all/team). **Enter gives `ChatCommand::Send { text, team }`** (trimmed, never empty) and closes; Escape, or Enter on an empty draft, gives `ChatCommand::Cancel`. A closed chat ignores every key. The draw shows `[All] > text|` with the cursor, a window of the draft that follows the cursor when it is longer than a row.

## Nameplates

`Nameplates::layout_into(players, out)` / `draw_into` take teammates with a point above the head **already projected** by the caller (`Projected { screen, distance, in_front }`).

- Left out: behind the camera, not finite, off the canvas, or farther than 60 m. Of the rest the **8 nearest** are kept, nearest first (ties keep the list order).
- Up to 15 m a plate is full size and opaque; from 15 to 60 m it shrinks to 0.6 and fades to 0.35 linearly.
- The plate stands on the anchor (bottom edge) and is **moved inside the safe area** (5 % margins) if the anchor is near an edge.
- Name (cut to 16 characters; grey when dead), a health bar in proportion that is green above half, yellow above a quarter, red below, an armor bar only with armor, a green strip when talking. The farthest is drawn first so the nearest is on top. Health or armor that is NaN, negative, above the maximum or with a maximum of 0 is shown as 0 or full, never a crash.

## Ping markers

`PingMarkers::add(owner, name, kind, label, ttl_ticks)` (a player has one ping at a time, at most 16 live, a lifetime of 0 adds nothing), `update()` per tick, and each frame the caller gives `PingProjection`s (a world position found on the screen). A ping without a projection is not drawn.

- A target in front of the camera and inside the safe area gets a square in the kind's colour with a symbol, a label and "Owner, N m".
- Any other is **pinned to the edge of the safe area** on the line from the screen centre toward the target, with an arrow (one of eight). For a target **behind** the camera the projected position is mirrored, so the arrow points the way to turn.
- The last 30 ticks of a ping fade out. Label text goes on the side of the marker that faces the middle of the screen, and is kept inside the canvas.
- A position that is not finite is not drawn.

## Voice indicators

`VoiceIndicators::update(samples)` per tick with the level each player's voice has (0 to 1; below 0.02 is silence). A talking player is listed with a level meter; after the voice stops the row stays **20 ticks**, dimmed with an empty meter, so a pause between words does not blink. At most 6 are listed (holding speakers go first, then the quietest), ordered by id, at the right edge of the safe area. The scoreboard dot and the nameplate strip use the `talking` flag the caller sets.

## How it was tested

**118 unit tests** (`coop_overlays_tests.rs`, among them five property tests with arbitrary and non-finite numbers) cover every rule above and its edges: empty, one and 16 players, 17 and 20 (the cut, the "+N more" line, the local player moved into the last row), the sort (every tie-break, arrival order, negative scores), long, empty and control-character names, the ping colours at their exact limits; the chat (wrapping at a space, inside a word, exactly before a space, a zero width, the history limit over 1000 lines, the fade at the tick it starts and ends, zero fade, an old line not hiding a newer one, editing with multi-byte characters, the draft limit, send/cancel/team, a cursor window over a long draft); nameplates (behind the camera, off canvas, NaN and infinite positions, distance 0 / near / mid / far / beyond, near equal to far, the count limit, ties, every edge of the safe area, a safe area smaller than a plate or NaN, health and armor that are NaN, negative, infinite or with a maximum of 0); pings (lifetime to the tick, replacement, the limit of 16, every margin of the pin rectangle, the four edges, a corner, a steep line, behind the camera, the exact centre, NaN positions, a position so near the centre that the ray would overflow, text kept on the canvas); voice (the threshold, the 20-tick hold and its restart, the limit of 6 and who goes first); and the whole in its documented order.

**Mutation check.** I broke the code by hand 228 times, one exact-text change each (flipped comparisons, off-by-one limits, swapped operands, removed branches and clamps, changed constants and defaults, wrong colours, swapped draw order), and ran the module's tests after each. **189 failed a test at once; 39 survived.** Of those, **26 were real gaps** and got a test each (the panel height and the "+1 more" line, the zebra stripes, the cursor after closing, a tiny wrap width, the room the input line takes and the colour of each chat kind, wrapping after a one-letter word, the plate's bar widths and the fade of its box, an infinite ping distance, the order of label and details, a long label near the edge, the voice list's position, spacing and local strip, and the documented defaults); all 26 now fail a test. **Six were dead code** that I deleted (a clamp that can never act with 16 rows, a `max` that the arithmetic makes redundant, a finite check that the canvas test already implies, an unreachable branch for near >= far, a tie-break that the stable sort already gives, a "holding first" comparison that a level of 0 already gives). **Seven are equivalent or defensive and are left**: the clamp inside `faded` (every caller passes 0 to 1), `>` against `>=` where exactly 16 players need no cut, a `max_stored` of 0 (the trim loop empties the history anyway), a `continue` in place of `break` where the draft is already full or where older lines are always older, a fade that reaches 0 at the same value either way, and the fade of a meter that has no height while holding. A second pass over the code I changed afterwards (the stacking of ping text, the voice name length) was 8 for 8. Totals: **236 mutants run, 223 killed, 6 removed with their code, 7 equivalent or defensive, none unexplained.** Mutation testing was done on this one Windows machine by a script that is not part of the repository.

**Golden screens.** The five overlays are in the screen matrix of `docs/GOLDEN_IMAGES.md`, each at five sizes (twenty-five goldens, the largest 72 KB), drawn by the real `CoopOverlays`: the scoreboard with seven players (the local one highlighted, one talking, a bot with no ping, a long name, pings in all three colours and 999), the chat (an old line, a system line, a wrapped line, a team line, and an open team input with the cursor in the middle), nameplates (near, mid, far, low health, armor, dead, talking, one pushed in from the screen edge, one behind the camera that is not drawn), pings (an item and a danger on screen, one off to the right, one behind the camera, one off a corner and fading), and the voice list (four talking at different levels, the local one marked, one holding and dimmed). Looking at the first renders showed the voice name running to the edge of its row (names there are now cut to 14 characters), the local strip of the voice list the same orange as a focused menu button (now the scoreboard's local colour, so the golden-free "no focused button" check is meaningful), and a gap above a ping with no label (empty lines now take no room). The matrix passes on this machine's default, forced software, DirectX 12 and OpenGL adapters (see the pull request).

## Not covered (follow-up T10-014b)

- **Nothing feeds the overlays**: no game state gives players, kills or pings, no network carries chat, pings or voice (T08), no input mapping turns key events into `ChatKey`, `set_held` or `open`, and the app does not call `CoopOverlays` yet.
- **No real projection**: `Projected` is given by the caller; there is no camera, no world-to-screen code, no occlusion (a plate behind a wall is drawn), no head-height offset and no ping raycast.
- **No voice at all**: the indicator only draws levels it is given; there is no microphone, codec, transport, push-to-talk or mute (T08, T06).
- **Placeholder look**: flat quads and the Inter font, no icons for the ping kinds (they are one character), no team colours, no avatars or ranks, no animation beyond fades.
- **No measurement of text**: widths are estimated, so a very wide name could still overrun its column; wrapping is by characters, not by pixels, and right-to-left text is not handled.
- **Chat is not moderated**: no profanity filter, no mute list, no rate limit, no localisation of the labels ("Scoreboard", "Kills", "[Team]").
- **Not in a frame budget**: not allocation-free and not added to `docs/UI_BUDGET.md`.
