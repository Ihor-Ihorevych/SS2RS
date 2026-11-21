# RFC 0004: Audio API (contract v0)

- **Number:** 0004 (`docs/rfcs/0004-audio-api.md`)
- **Authors:** @lxstyo
- **Status:** in review
- **Approvers:** owner of `ss2-audio` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T00-018 (this contract); unblocks T06-001..006, T06-015/016/017/018 (mixer, 3D, voices, music, replicated events, hooks, buses, mocks) and every caller (T09 weapons/enemies, T10 UI sounds, T03 lifecycle)

## Summary

Add a small crate `ss2-audio-api` that defines everything gameplay, UI and the app may ask of audio: resolve a sound path to a `SoundId`, `play` it with `PlayParams` to get a `VoiceId`, move/re-level voices, set the 3D `Listener`, crossfade a music track, set per-`Bus` volumes, and suspend. No `cpal`, decoder or device appears in it. Ship `NullAudio`, a headless implementation that models voice ids, priority culling, buses, music and listener, so callers are written and tested now.

## Motivation

Weapons, enemies, pickups, UI and music all make sound, while the mixer (T06-001) and the original game's attenuation rules (T01-034) are still to come. Without a frozen API each caller would invent one or block. Keeping the contract device-free also lets the headless dedicated server (no audio device) run the same game code through `NullAudio`.

```
ss2-game / ss2-ui / ss2-app ──► ss2-audio-api ◄── ss2-audio (cpal mixer, decoders)
```

## Detailed design

All code is in `crates/ss2-audio-api/`; the rustdoc examples run as tests.

### Resolving sounds

`AudioResources::sound(&mut self, path: &str) -> SoundId`: cheap and idempotent (same normalized path, same id), and **valid immediately even while loading or after a failed load**: playing an unloaded or broken sound is silent, never an error for the caller (load errors surface through `ss2-assets`). A `.scs` sound collection is also a `SoundId`; which variant plays, and pitch/volume jitter, are the audio system's job (T06-004). Same pattern as `RenderResources` (RFC 0002).

### Playing

`Audio::play(sound, &PlayParams) -> Result<VoiceId>` with `PlayParams { volume (linear, >= 0), pitch (rate 0.1..=4), bus, priority, looped, spatial: Option<Spatial> }`. `Spatial { position, velocity, min_distance, max_distance }`: full volume inside `min`, silent beyond `max`; the curve in between is the audio system's business (T01-034). No `spatial` means 2D (UI, music, own-weapon sounds). Presets (`PlayParams::sfx()`, `ui()`) and chainable builders keep call sites short. Invalid params (NaN, bad ranges) return `Err` and play nothing; **everything else is infallible for the caller**.

### Voices

`VoiceId(u64)` is never reused within a run, so a stale id is harmless: `stop`, `update_voice` and `is_playing` on a finished, stopped or culled voice are silent no-ops / `false`. Under voice pressure (mobile budget, T06-006) the mixer culls lowest `Priority` first; **a culled voice still gets a `VoiceId`** (it is simply not playing), so callers never handle "no voice available". `update_voice(voice, &VoiceUpdate)` moves, re-levels or re-pitches (per frame, for emitters attached to entities). Looped voices end only through `stop(voice, fade)`.

### Listener, music, buses, suspend

- `set_listener(&Listener { position, velocity, forward, up })`: once per frame from the camera; an unusable orientation (zero or parallel vectors, NaN) is rejected and the previous listener kept.
- `play_music(track, crossfade)` / `stop_music(fade)` / `current_music()`: one looping track, crossfaded; asking for the current track is a no-op. Level-state-driven layers and cues (T06-005) are built inside the audio system on top of this.
- `Bus::{Master, Music, Sfx, Voice, Ui}` (`#[non_exhaustive]`) with `set_bus_volume` (clamped `0..=1`, NaN ignored) and `bus_volume`; `Master` scales the rest (T06-017).
- `set_suspended(bool)` mutes while the app is suspended or unfocused without dropping voices, driven from the platform lifecycle (RFC 0003).

All methods return quickly and never block on the device; changes apply from the next mixer block.

### The mock: `NullAudio`

Full state (voices and their parameters, listener, bus volumes, music, suspend flag) plus an ordered `calls()` log (`Play`, `Stop`, ...). `with_voice_limit(n)` models culling (a higher-priority newcomer evicts the lowest-priority voice; an outranked or equal newcomer is itself culled; no churn between equals). `end_voice` simulates a one-shot finishing. `MockAudioResources` hands out sequential ids and maps them back to normalized paths for assertions.

### Not in the contract (deliberately)

Replicated audio events and "who hears what" in co-op (T06-015, depends on the net contract); animation-event hooks (T06-016); reverb zones (T06-007); voice chat (T06-014); decoders and streaming policy; the offline renderer (T06-013); output device selection.

## Alternatives considered

- **Entity-attached emitters inside the audio system** (the audio system tracks entities itself). Rejected: it would couple audio to the sim's entity model (T00-017, not yet defined); `update_voice` per frame is simple and keeps the API self-contained.
- **A `Result` for "no voice free".** Rejected: every caller would have to handle it identically (ignore). Returning a non-playing `VoiceId` removes a whole class of branches.
- **Sound handles as asset `Handle<T>`s (RFC 0001).** Rejected for v0, same reasoning as RFC 0002: the decoded-sound asset type does not exist yet, and ids are cheaper and simpler for per-event calls.
- **Bus effects/ducking in the API.** Deferred: ducking is a mixer policy (T06-006), not a caller concern.
- **Typed `Volume`/`Pitch` newtypes.** Rejected: validated `f32` fields with presets read better at call sites and the one place that checks them is `validate()`.

## Compatibility and migration

- New crate only; no existing code changes. `ss2-audio` (T06) depends on it and implements `Audio`. The design doc crate map gains `ss2-audio-api`.
- After merge the signatures are frozen; compatible additions (new `Bus` variants, new default methods) need owner approval, anything else a new RFC.

## Open questions

- **Fade curve and crossfade shape** (linear vs equal-power): decided by the `ss2-audio` owner in T06-005.
- **Per-voice bus override and send levels / reverb sends**: decided with T06-007.
- **Doppler strength and rolloff exponent as settings**: decided with T01-034's findings.
- **Sound-finished callbacks** (e.g. for subtitles or scripted sequences): polling `is_playing` works for v0; a completion event may be added with T06-016.
