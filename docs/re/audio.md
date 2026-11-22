# Audio semantics: sources, ranges, environments, music, assets (T01-034)

- **Roadmap task:** T01-034
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the asset formats (every `.wav` and `.ogg` header read), for the classes and their members and for the values quoted (read from the global databases and from five worlds); **medium** for the meaning of members that have readable names (hot spot, fall-off, volume, pitch); **low** for numbered members and flag bits; **open** for the attenuation curve, priorities and voice limits, the reverb presets and the music state machine: none of these is in a data file.

Prerequisites: `mat.md` (sound schemes `.scs`), `classes.md`, `wld-root.md` (sound sync lists), `ai.md` (`CBAPlaySound`), `rsc-ctl.md`.

## Summary

Sound in the original is **3D positional with a two-radius model** (a *hot spot* radius inside which a sound is at full volume and a *fall-off* radius at which it is no longer heard), sources and channels that live in the world, **environment boxes** that switch a reverb preset, a **dynamic music** object that crossfades between two streams, and **sound schemes** that map game events to weighted sound alternatives. All sound files are plain PCM `.wav` (short effects, voices) or Ogg Vorbis `.ogg` (music, long voice-overs). The shape of the attenuation curve, the number of simultaneous voices and the reverb presets are runtime behavior and remain open.

## Evidence

`tools/re/audio_verify.py` reads the headers of all sound files (numbers only). The classes and members come from the type tables (`classes-catalog.md`); the values come from `GlobalGameParameters.rsc` (`rsc-ctl-schema.md`) and from decoding the five smallest worlds (`dockingbay`, `dm_stadium`, `dm_branchester`, `1_3_village`, `dm_yodeller`; 1,519 sound channels). Nothing from binaries.

## Asset formats

| Format | Files | Facts |
|---|---|---|
| **WAV** | 1,868 | all **PCM, 16-bit** (format tag 1); **1,861 mono** and 7 stereo; sample rates 44,100 Hz (1,601), 22,050 Hz (254), 11,025 Hz (10), 48,000 Hz (2) and one at 33,333 Hz; 4,205 s (about 70 minutes) in all |
| **Ogg Vorbis** | 392 | all 44,100 Hz; **314 mono, 78 stereo**; 317 files under 100 KB, the rest 1.4 to 2.4 MB (music); where: `Sounds/Netricsa` 202, `Sounds/Player` 103, `Music/*` 77, `Sounds/Npc` 9 |

WAV folders (count): cut-sequence voices 485 (1,693 s: the longest group), enemies 456, ambient 150 (788 s), materials 139, bosses 123, NPC 92, weapons 86, player 66, misc 51, impacts 43. **Music** is organized by chapter folder (`01_mdigbo` 20 tracks, `05_ellenier` 13, `07_sirius` 9, `04_kleer` 8, `02_magnor` 7, `03_chifang` 6, `06_kronor` 6) plus `multiplayer` 3, `nettie` 2, `arenas`, `boss`, `menu`; track names end in a **mood**: `medium` 21, `peace_tense` 15, `peace_relax` 15, `fight` 7, `peace` 7, and single tracks `enemies_approaching`, `boss_fight`, `success`, `fail`, `finished`, `continuous`. That is the vocabulary of the **dynamic music** (calm, tense, medium, fight, approaching, boss, jingles).

Almost all effects are mono, as positional (3D) sources need; the 7 stereo WAV files and the 78 stereo Ogg files are music or non-positional sounds.

## Sources, channels, listeners (world data)

A **sound** in a world is a pair: a `CSoundSource` (a placed aspect) and its `CSoundChannel`. 1,519 pairs in the five sampled worlds.

| `CSoundChannel` member | Observed | Meaning |
|---|---|---|
| `.1` `SndFlags` | 0 (1,477), 3 (40), 1, 7 | flag bits; 3 and 7 are looping or 3D variants (unconfirmed), `mat.md` lists more values |
| `.2` feeder | null for 1,477, set for 42 | a `CSoundFeeder` for streamed sounds |
| `.3` sound pointer | | the `CBaseSound` (a `.wav` or `.ogg`) |
| `.4` sync list | | the world's `CSoundSyncList` |
| `.10` | 1 (1,479), 0.5 (38), 0.4, 0.3 | **volume** |
| `.11` | 0.99 to 1.007 | **pitch** (a random offset around 1) |
| `.20` | 10 (727), 5 (655), 1 (62), 0 (60), 30 (6) | **hot spot** (full volume up to this distance) |
| `.21` | 50 (621), 75 (594), 150 (114), 5 (35), 0 (60) | **fall-off** (the distance where it stops) |
| `.22` | 1 (1,516), 0 (3) | a flag (3D on/off or "loop"), unconfirmed |
| `.33`, `.34` | 360 (1,518), 120 (1) | **cone angles**, degrees: 360 is omnidirectional (inner and outer cone) |
| `.25`, `.26` | placement, velocity | position and orientation of the channel, velocity vector (0) |

The names match the particle-effect sound class (`CParticleEffectSound`: `pes_fVolume`, `pes_fPitch`, `pes_fRndPitch`, `pes_fHotSpot`, `pes_fFallOff`, `pes_ulFlags`), which fixes the meaning of the two radii. `CSoundSyncList` (the world's sound hub, `wld-root.md`) holds the list of channels, an array of **listeners** (`SoundListener`), the environment boxes and a time stamp with two booleans.

A **static sound entity** (`SStaticSoundEntity`, properties `SStaticSoundProperties`, 11 in the sample) is an editor-placed ambient sound: names such as `Sound_Crickets`, `Sound_Monkeys`, `Sound_CrowdAmbience`, `Sound_NightAmbience`; volume 1 (6), 0.5 (3), 0.4, 0.3; a random pitch range 0.01 to 0.05; a **hot spot** 1 to 500 and **fall-off** 20 to 500 (50 and 33.33 most common); cone angles 360 (or 120); the same `SndFlags` (3 ×5, 2 ×4, 1, 7). A **hot spot of 1 to 4 with a long fall-off** means "a point source heard from far away"; a hot spot equal to a large fall-off share means "an area ambience".

### Global ranges (`GlobalGameParameters.rsc`, meters assumed)

| Group | Hot spot | Fall-off |
|---|---|---|
| Puppet sounds | 10 | 150 (50 for other puppets) |
| Moving sounds | 10 (5) | 125 (50) |
| Step sounds | 1 or 5 | 25 |
| Weapon sounds | 2 | 60 |
| Door sounds | 1 | 50 |
| Projectile flying | 1 | 30 |
| Projectile explosion | 1 | 90 |
| Destruction | 5 | 75 |
| Bullet impact | 1 | 30 |
| Dust | 1 | 20 |
| Thrust | 1 | 35 |
| Saw | 1 | 35 |
| Navigation beacon and objective fade-out | 20 | 40 |

(The two puppet sets are `gpp_psr` and `gpp_psrOther`, the second for other characters.) These numbers are the starting point for a **distance attenuation** that is full volume inside the hot spot and silent beyond the fall-off; **the curve in between is not in the data** (linear in distance and inverse-distance are the natural candidates; to be measured, see open questions).

## Environments (reverb zones)

`CSoundEnvironment`: a box (`Box3f`, six floats), an **environment preset** (`SfxEnvironment`, an enumeration; values seen in the sample: 0 and 6; 44 worlds have such boxes) and a pointer to the sync list. The class name (`Sfx…`) and the install's Windows audio modules (`SfxDSD.dll` and `SfxOAL.dll`; the names suggest DirectSound and OpenAL back ends) point to **EAX-style reverb presets** selected by the listener's position. The names of the preset values are not in the data.

## Dynamic music

`CDynamicMusic` (one per world, 59 of 59): the current **music sound** (`dm_pbsMusic`, a `.ogg`), a volume multiplier, a **fade-in time**, the index of the active source and an array of **two** `CDynamicMusicSource` (each: a source, a desired volume, a current volume and the volume at the start of the fade). Two sources allow a **crossfade** between the outgoing and the incoming track; which mood plays when (peace, tense, medium, fight, boss) is driven by game events (the macro language and the game module) and is not stored. The track names above give the moods that exist per chapter; a level has several moods (for example the village level `1_2` has relax, tense, medium and enemies-approaching tracks).

## Event sounds: schemes and animation events

Sounds that belong to characters and materials are not placed but **requested by term** (`mat.md`: sound scheme `.scs`, 37 files; the class occurs in 449 files): the term (for example `Wound`, `Sight`, a material impact) selects among weighted alternatives (`CWeightedSoundInScheme`, 7 members: weight, sound, flags, pitch, variation, volume), inherited from a base scheme. Behaviors play sound terms (`CBAPlaySound`: volume, pitch, random pitch; `ai.md`), animations carry **sound events** (`CSamSoundAnimEvent`, `SPuppetSoundAnimEvent`: footsteps and so on, `help-digest.md`), destruction effects have `CDFXSound` (11 members, in 464 files) and spawn effects a sound (`.sfc`).

## What is not in the data

1. The **attenuation curve** between hot spot and fall-off, and any distance filtering (high-frequency roll-off).
2. **Priorities and the voice limit**: the number of simultaneous sounds and which one is dropped (the engine has a sound mixer in the Sfx modules; no priority member exists in the classes above).
3. Doppler (the channel has a velocity vector, `.26`, 0 in all samples) and the rules for panning and spatialization.
4. The **reverb preset table** and its mapping to parameters.
5. The **music state machine** (who requests which mood, how long fades are).
6. Streaming: which sounds stream (the feeder, `.ogg`) and buffer sizes.

For a reimplementation: model positional sources with `{volume, pitch, hot spot, fall-off, cone inner/outer}`, a listener list, environment boxes returning a preset id, two-slot music crossfade and term-based schemes; then tune the curve against recorded game audio (T01-038) once available.

## Reader invariants

1. `.wav` files are RIFF PCM 16-bit; read the `fmt ` and `data` chunks, accept odd-length padding.
2. Ogg files are Vorbis, mono or stereo, 44,100 Hz.
3. Do not assume that the hot spot is smaller than the fall-off: 35 channels have a fall-off of 5 next to hot spots of 5 or 10, and 60 channels have zeros (probably "no distance attenuation"); a reader must accept both.
4. Cone angles of 360 mean "omnidirectional".

## Open questions

The "What is not in the data" list, plus: the bits of `SndFlags`, member `.22`, and the values of `SfxEnvironment`.

## Test vectors

`tools/re/audio_verify.py` prints: `.wav` 1,868, `.ogg` 392; wav format tag 1 ×1,868, channels 1 ×1,861 and 2 ×7, bits 16 ×1,868; ogg channels 1 ×314 and 2 ×78, rate 44,100 ×392. Global ranges: puppet sounds hot spot 10 and fall-off 150; weapon sounds 2 and 60; door sounds 1 and 50.
