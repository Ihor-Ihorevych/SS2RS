# Subtitle and caption timing

`ss2_ui::subtitles` (T06-012) is the timing model behind subtitles and captions, with nothing drawn and no clock: a **queue of cues**, each shown for **at least the time it takes to read it**, with **fades**, a **limit on simultaneous lines**, **priorities**, **late and out-of-order arrival**, and **pause, seek and skip** for cutscenes. The caller owns the timeline: it reports the time that passed, or jumps, and reads what to show.

```rust
let mut subs = Subtitles::new(SubtitleConfig::default());
subs.add(Cue::new("Here they come!", 12.0).speaker("Sam").priority(1))?;  // start on the cutscene's timeline
subs.advance(dt);                       // every frame, with the time that passed (nothing moves while paused)
for line in subs.visible() {            // oldest first; line.row == 0 is the newest, at the bottom
    draw(line.text, line.speaker, line.row, line.alpha);
}
let style = settings.subtitle_style([1.0; 3]);   // size, box and contrast: ss2_ui::accessibility (T10-010)
let box_alpha = line.box_alpha(&style);          // the player's box opacity times the fade
```

## The rules

- **Reading time.** A cue is shown for `max(its duration, reading time)`. The reading time is the number of words (or, if more, the number of six-character groups, so text written without spaces is not "one word") divided by `words_per_second` (3 by default), and never less than `min_duration` (1 s). A duration of 0 means "as long as it takes to read".
- **Fades.** The alpha rises over `fade_in` (0.15 s) and falls over `fade_out` (0.25 s). Each fade is shortened to half of a very short cue so that the two never overlap. The alpha is always in 0 to 1.
- **Lines.** At most `max_lines` (3, at most 8) cues are shown. `visible()` lists them oldest first; `row` 0 is the newest and older lines have higher rows (they are pushed up as new ones arrive).
- **A full screen.** A cue that starts when every line is taken **evicts** the lowest-priority cue (the one shown longest among equals, then the one added first) if its own priority is *strictly* higher. Otherwise it **waits** for a line for up to `max_wait` (1.5 s) and is dropped after that. A waiting cue is shown for its whole time from the moment it gets a line, so it is never cut short.
- **Late and out-of-order cues.** Cues can be added in any order (they start in time order, then priority, then order added). A cue added after its start time is moved to now if it is at most `late_grace` (0.5 s) late, and **refused** if it is later. Bad cues (empty text, not-a-number times, a negative duration) are refused; so is a cue when `max_pending` (64) are already waiting.
- **No dependence on step size.** Time is processed event by event, so one step of ten seconds and a thousand of 10 ms give the same cues starting and ending at the same moments (a property test).
- **Pause.** While paused, `advance` does nothing: the alphas stay where they are. Adding, seeking and skipping still work.
- **Seek forward** (a cutscene skip to a later point): cues that would have ended are dropped as `Skipped`; cues the jump lands inside are shown for the **rest** of their time without fading in again; later cues stay queued. If the jump lands in more cues than there are lines, priorities decide as above and the rest are skipped.
- **Seek backward:** cues that had started after the new time go back to waiting (`Rewound`) and start again; cues that already finished are **not** brought back.
- **Skip all** drops everything, shown or waiting, as `Skipped` and says how many that was.
- **Events.** `take_events()` lists what happened since the last call (`Started`, `Finished`, `Dropped(reason)`, `Rewound`) with the time on the timeline, so a caller can log, test, or react (for example to hide a speaker portrait). Up to 256 are kept.

The player's choices (size, box, high contrast) are **not duplicated here**: `AccessibilitySettings::subtitle_style` (T10-010) decides them, and `SubtitleLine::box_alpha` only combines the box opacity with the fade.

## How it was tested

Sixty-five unit tests, six property tests (400 random cases each) and one doctest. The unit tests cover: the reading time (words, six-character groups, the minimum, the speed), the effective duration, a cue's exact start and end whatever the step, the fades (up, down, shortened for short cues, either one off), the box alpha with the player's style, the line limit, waiting and giving up at the exact moment, evictions (strictly higher priority only, the lowest and oldest first, ties), rows, late cues (the grace limit and where the wait counts from), out-of-order arrival, refusals (empty text, NaN or infinite times, negative durations, a full queue), bad steps, pause, forward and backward seeks (landing inside, exactly on an end, past everything, with more cues than lines), skipping, id reuse, the configuration's clamping, and the event buffer. The property tests (random cues, steps, pauses, seeks and skips): the screen never holds more than `max_lines`; rows are 0..n once each; alphas are within 0..1; without seeks every finished cue was shown for exactly its effective time, which is at least the minimum; every accepted cue ends exactly once; with one line and equal priorities no two cues overlap; started and ended events balance across seeks and skips; the result is deterministic; and one large step equals many small ones.

**Mutation checks.** About ninety deliberate one-line breakages (comparisons flipped, off-by-ones, clamps dropped, constants changed, orderings reversed) were applied one at a time and the module's tests run. Real gaps that survived at first, each now covered by a test or by simpler code: the late-shift of a cue (where the wait counts from), three eviction tie-breaks (shown longest, then lowest id), a redundant `max` in the step loop and two redundant zero-fade branches (**removed**: the NaN/infinite ratios already do the right thing and a test covers each fade being off), a seek landing exactly on the end of a cue on screen, the lower clamp of the box alpha, and the upper clamp of `fade_out`. **Three survivors are equivalent and remain:** `dt <= 0.0` against `dt < 0.0` (a step of exactly zero changes nothing either way), `to >= now` against `to > now` in `seek` (a seek to the current time does nothing in either branch), and the insertion test `is_lt` against `is_le` (ids are unique, so two cues never compare equal). One more "survivor" was my script's own no-op mutation. Some mutants ran until a 60 s timeout (an endless loop in the step loop) and count as killed.

## Not covered (follow-up T06-012b)

- **Nothing draws it.** No text, outline, box or layout (wrapping, width, where on the screen) is drawn; the model only says what is shown, where in the stack and how opaque.
- **There is no real audio or video clock.** The caller must supply the time; nothing here follows the audio mixer or the video player (T06-010 and T06-011 do not exist yet), so lip-sync drift and a stalled stream are not handled.
- **There is no subtitle file reader and no game data.** Where the cues come from (the localization tables and the subtitle sync in the T01-041 spec, the cutscene scripts) is not wired: cues are made in code. The cue texts, speaker names and their localization (T10-011) are the caller's.
- **No rewinding of finished cues, no caption sound descriptions** (`[explosion]`), no per-language reading speed, no line wrapping that counts the screen width, and no text-to-speech.
- **Numbers are mine.** The reading speed (3 words a second), the minimum time, the fades, the grace and the waiting time are common defaults, not measured on the original game or tuned on a screen.
- **An evicted cue vanishes at once** (no fade-out), and a cue the player seeks into is shown only for what is left of it, below the minimum reading time if that is all there is.
