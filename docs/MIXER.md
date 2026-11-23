# The mixer core

`ss2_audio::mixer` (T06-006, T06-013, T06-017) is the **sample arithmetic** of the audio system: voices, buses, ducking, voice management and the sum, with **no audio device and no decoder**. A device callback (T06-001) or the offline renderer calls
`Mixer::render(&mut [f32])` for each block of interleaved stereo frames; decoded samples come from `SampleBuffer`s (mono or stereo `f32` with their own rate) registered under a `SoundId`. It uses the types of the audio contract
(`ss2_audio_api`: `PlayParams`, `Bus`, `Priority`, `VoiceId`, `Listener`, `VoiceUpdate`; RFC 0004).

## Voices

`play(sound, &PlayParams)` returns a `VoiceId` (ids are never reused, so a stale id is harmless). A voice plays its buffer with a **volume**, a **pitch** (the playback rate, resampled by **linear interpolation**; a source at another sample rate plays at the right
speed), a **bus**, a **priority**, optional **looping** (the cursor wraps by the overshoot, so a fractional step does not drift) and an optional **position**:

- a **2D** voice is centred with an **equal-power** pan (each channel at 0.7071, so a centred sound is not louder than a panned one);
- a **spatial** voice is attenuated **linearly** from `min_distance` (full volume) to `max_distance` (silent) and panned by where it is relative to the listener (the listener's right is `forward x up`; there is no front-back cue). The linear curve is
  a placeholder: the original game's curves are T01-034 / T06-003.
- gains change by a **ramp across each block**, a new voice fades in over its first block, a stop fades out (`stop(voice, fade_ms)`, never shorter than the declick time, 10 ms by default), and **suspend** (`set_suspended`) fades the output to silence and back over the declick
  time **without dropping voices**: nothing clicks (tested at half way through each ramp, in blocks smaller than the ramp too).

## Buses and ducking (T06-017)

`set_bus_volume(bus, v)` for `Master`, `Music`, `Sfx`, `Voice`, `Ui`: clamped to 0 to 1, NaN ignored; a voice hears **its bus times the master** (a voice on the master bus is scaled once). **Ducking** lowers the `Music` bus (default to 30 %, 50 ms attack, 400 ms release, exponential) while any `Voice` bus voice
is **audible** (a muted one, or one beyond its range, does not duck); a music gain of 1 turns it off. Only music is ducked.

## Voice management (T06-006)

At most `max_voices` voices are mixed (32 by default: **a mobile tier sets fewer**; the configuration is the tier). The others are **virtualized**: they keep advancing as if playing (so they are in the right place when they return) but cost no mixing, and a virtualized one-shot still ends on time.
Who stays audible: **higher priority first, then the louder (volume x distance x bus), then the older voice**. `Priority::CRITICAL` voices are **never virtualized by pressure and do not count against the limit** (the contract says "must never be culled"); a voice that is silent (beyond its range, or volume 0 and not critical)
is virtualized whatever the limit. At most 1,024 voices may be alive; `play` refuses the next.

## The offline renderer and golden audio (T06-013)

`render_offline(&mut mixer, frames, block)` renders a number of frames in blocks into one vector; `wav::encode_pcm16` / `encode_f32` write it (`wav::decode` reads back what the renderer writes, for tests; it is **not** the game's `.wav` decoder, T06-002).
`compare(a, b)` and `AudioTolerance` are the audio counterpart of the golden-image tolerance: the **largest single difference** (a click or a dropout) and the **RMS difference** (a level change spread over everything), default 1e-4 and 2e-5. **Deterministic**: voices are visited in id order and all arithmetic is
`f32` (the cursor `f64`), so the same calls give the same samples on one machine and build (a test compares bit for bit); across architectures the last bits may differ, which is why golden tests use a tolerance. The golden test in the repository has **no stored file**: two sines in two places are rendered and compared with the
sum computed independently from the source samples; stored reference files are for scenes that have no closed form (follow-up).

## Cost

One machine (12th Gen Intel i5-12450HX, release, one thread), linearly interpolated mono sources with pitch and position, blocks of 512 frames (10.7 ms): **8 voices 20 us per block (0.2 % of the time the block lasts), 32 voices 83 us (0.8 %), 128 voices 325 us (3.0 %)**. Not measured on a phone, and the 3D and music parts do not exist yet.

## How it was tested

Seven unit tests (wav, compare) and **36 tests of the mixer** with analytic expectations (constants and sines whose output can be computed by hand): the centre gain, the first-block fade-in, a sine through the mixer within the golden tolerance, pitch by interpolation at 2x and 0.5x (on a ramp steep enough for the tolerance to see interpolation),
a source at another rate, stereo sources, one-shots and loops, panning left, right, ahead, behind and inside the head, the listener turning, linear attenuation at five distances, buses and master, clamping, ducking down and back, only music ducked, silent and critical-muted voice-overs not ducking, the voice limit by loudness, by priority, critical never virtualized, ties to the older voice,
a virtualized voice back in sync, a virtualized one-shot ending, the alive limit, stops with and without a fade, suspend and resume (including blocks smaller than the ramp), bad requests, odd buffers, clipping, determinism and the renderer. **Twenty-five deliberate breakages: twenty each failed a test at once, six survived my first tests** (the declick minimum, ducking other buses,
the resume ramp speed, a muted critical voice-over, loop drift, and interpolation), and I added tests until each failed; the others are equivalent at the boundary (a distance exactly at the minimum or maximum, or a time constant of 0). **A real bug was found by the tests while writing them:** the suspend ramp subtracted a step every block when already at its target, so every voice
slowly faded out; the first run of the tests showed levels at 79 % of the expectation.

## Not covered (follow-ups T06-006b, T06-013b, T06-017b)

- **The device and its thread** (T06-001: `cpal`, buffer sizing, device changes), **decoders** (T06-002), **streaming** (T06-009), **music** (T06-005), **reverb** (T06-007), the Doppler effect and occlusion, and the original's attenuation curves (T06-003).
- **`impl Audio` for the mixer** (T06-017b): the contract's `play(sound, ...)` resolves sounds by path (`AudioResources::sound`), which needs the sound bank; the mixer takes buffers by id for now. The Android pause/focus event wiring (`set_suspended` is the hook) and the settings screen and file (T10-012).
- **Voice tiers wired to quality settings** and **mobile cost** (T06-006b): `max_voices` is a parameter; nothing sets it from a tier, and nothing was measured on a phone.
- **Stored golden files** and a cross-architecture check of the tolerance (T06-013b).
- **A limiter** (the output is hard-clamped and the clipped samples are counted; a soft limiter would sound better).
