# Static sounds, vibration, environment sounds, movie players and flares of `Sam2Game.dll` (T01-110, part of T01-026)

- **Roadmap task:** T01-110 (overview [`worldcontent-classes.md`](worldcontent-classes.md); this page covers `CStaticSoundEntity`, `CStaticVibrationEntity`, `CEnvSFXEntity`, `CMoviePlayerEntity`, `CFlareEntity` and their properties classes, 10 classes)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the member tables, the counts, the copies from record to entity and the state values (`tools/re/worldcontent_classes_verify.py`); **medium** for the state machines, net rules and flare clamps (one reading each; **nothing was run**); **low** for methods marked `P` or `N` and for the three classes that no world places.

Prerequisites: [`worldcontent-classes.md`](worldcontent-classes.md), `wld-sound-effects.md` and `wld-sound-effects-schema.md` (the sound environment data and the editor's `SfxEnvironment`), `audio.md`, `movie_verify`/`cutscenes.md` (movies), `lighting.md` (flares are `CFlareRenderable`), `replication.md`.

## Summary

| Class (data name) | Entities in the data | What it is |
|---|---|---|
| `CStaticSoundEntity` (`SStaticSoundEntity`) | **610** in 50 worlds | a sound source at a place, auto-playing or driven by macros, with fades |
| `CEnvSFXEntity` | **136** in 44 worlds | a box that applies a sound **environment** (reverb preset) to listeners inside |
| `CFlareEntity` (`SFlareEntity`) | **2,501** in 48 worlds | a lens flare / glow attached to a model |
| `CMoviePlayerEntity` | 0 | plays a movie on a texture and a sound |
| `CStaticVibrationEntity` | 0 | plays a controller-vibration pattern |

1. **A static sound is replicated only when a macro drives it** (static unless auto-play), and then through one member, `en_sssNetState` (overview, section 4).
2. **Environment sound effects have no code except boot-time parenting**: the entity creates the sound environment object from its record and the audio engine does the rest.
3. **The flare record is clamped on creation**: hot spot at least 0, fall-off at least the hot spot, coverage at least 0.01, glare between 0 and 1 (**all 2,501 records already satisfy it**, script).
4. **The movie player and the static vibration are in no world**: nothing in the DLL creates them except the editor's class registration (section 4, 5; T01-181).

## Evidence

- **Export table.** Static sound 30 + 10 symbols, static vibration 19 + 8, environment sound 11 + 11, movie player 14 + 8, flare 16 + 8: **135**, 65 named.
- **Member tables.** `SStaticSoundEntity` (19) and `...Properties` (13), `CEnvSFXEntity` (4) and `CEnvSFXProperties` (5), `SFlareEntity` (4) and `SFlareProperties` (7) are saved and match the data (script section 2). `SMoviePlayerentity` (6) and `SMoviePlayerProperties` (4) are declared but not saved; `CStaticVibrationEntity` has **no member table** (its members are visible only in the constructor and functions).
- **Methods.** Read: the static sound's `GetNetBehavior`, `PreSendUpdate`, `PostReceiveUpdate`, `GetRelevantDistance`, `SetState`, `OnBoot`, `OnStep`, `Play`, `PlayOnce`, `PlayLooping`, `Stop`, `OnShutDown`, `StatePlayOnce`, `StatePlayLooping`, `StateStopLooping`, `OnPlayWaitingSoundBegin`; the flare's `OnBoot`, `OnShutDown`, `ExecuteDestruction`, `GetCoverage`, `GetBoundingBox` and its `OnCreate` clamps; the environment sound's `OnBoot`, the properties' `OnStretch`, `OnGetStretch`, `GetBoundingBox`; the movie player's `PlayOnce`, `Pause`, `Stop`; the vibration's `OnBoot`, `PlayOnce`, `PlayLooping`, `Stop`, constructor. Profiled: the `OnCreate` functions, `RenderRanges`, `OnUpdate`.
- **Not checked:** nothing was run; the sound flags `SndFlags`, the 20 `SfxEnvironment` presets, `CVibroSource`, `CMovieChannel` and the editor drawing (`RenderRanges`) were not read.

## Layout / rules

### 1. `CStaticSoundEntity`

**Members** (19; **R** replicated). The record's members have the same order (13 members).

| # | Binary name | Meaning | Data (610) |
|---|---|---|---|
| 1 `@` | `en_hss` | the sound source | set |
| 2 `@` | `en_hre` | editor renderable (a cone or sphere showing the ranges) | set |
| 3 | `en_pbs` | the sound resource | set in all |
| 4 | `en_ulSoundFlags` | `SndFlags`; bit 0 means looping | 3 (170), 7 (163), 2 (139), 6 (124), 0 (5), 1 (4), ...; equals the record's `ep_ulFlags` in all |
| 5 | `en_bAutoPlay` | start by itself | 1 in 266, 0 in 344; equals the record |
| 20..23 | `en_fPitch`, `en_fPitchRnd`, `en_fVolume`, `en_fFallOff` | pitch (0.25..1.5), random pitch (0..0.5), volume (0.1..1), fall-off (4..100,001) | equal the record in all |
| 6 | `en_fSoundLength` | length of the sound | 0 in all (set at play) |
| 7 | `en_mlidMacroLatentID` | the macro blocked on `PlayOnceWait` | structure |
| 8, 9 **R** | `en_fFadeInLength`, `en_fFadeOutLength` | fade lengths of the waiting play | 0 in all |
| 13 | `en_tmStartTime` | when the current play started | structure |
| 14 | `en_sssState` | `StaticSoundState`: `SSS_NONE` 0, `SSS_PLAY_ONCE` 1, `SSS_PLAY_ONCE_WAIT` 2, `SSS_PLAY_LOOPING` 3, `SSS_STOP` 4, `SSS_STOP_LOOPING` 5 | 0 in all |
| 18 **R** | `en_sssNetState` | the net copy | 0 in all |
| 15, 16 | `en_henParent`, `en_idParentPart` | attachment | parent set 86 (all on models), part 0 |
| 17 | `en_fDelayPlay` | the auto-play delay | 1 in 606, 2 in 4; equals the record |

**Record** (labels): "Sound", "Volume" (default 1), "Pitch" (1), "Random pitch" (0.01), "Hot Spot" (10; 0..100,000), "Fall Off" (100; 4..100,001), "Cone angle" (360; 120 in 6, 90 in 1), "Soft angle" (360), "Outside cone volume" (0; 1 in 441), "Flags" (3 by default), "Autoplay" (1 by default), "Delay autoplay" (1), "Parent part". `OnStretch(s)` (editor) sets one range member to |s| and scales the other by the same ratio, so hot spot and fall-off keep their proportion (P); `OnGetStretch` returns that member on all three axes.

**State machine (R).** `SetState(n)` stores the state and runs: 1 `StatePlayOnce`: clamp the volume to 0..1, set the pitch, play the sound **non-looping** (flags with bit 0 cleared) and schedule `Stop` after the sound's length; 2 `OnPlayWaitingSoundBegin`: start the step handler, remember the start time, play non-looping, **clamp the fade-in to the sound's length (the fade-out is clamped too, P)**, schedule the end; during `OnStep` the volume is `min(elapsed / fadeIn, remaining / fadeOut, 1)` times the volume; 3 `StatePlayLooping`: play **looping** (bit 0 set) and remember the start time; 4 stop the channel; 5 `StateStopLooping`: **let the current loop finish**, scheduling the stop at the next multiple of the sound length. `OnBoot` (R): attach to the parent part's aspect if there is a parent; **if auto play is on, schedule the start after a random fraction (0 to 1) of `en_fDelayPlay` seconds**. `OnShutDown` stops the channel. The macro functions `PlayOnce`, `PlayOnceWait` (blocks the macro until the end), `PlayLooping`, `Stop`, `StopLooping` call `SetState(1, 2, 3, 4, 5)`; **each prints `Can't execute macro command "..." on static sound entity (ID %1) that has enabled AutoPlay option.` and does nothing when auto play is on** (R). `PostReceiveUpdate`: apply the net state when it differs; `PreSendUpdate` copies the state to the net state. `GetRelevantDistance` is the fall-off (R). `RenderRanges` draws the ranges in the editor (P). **Hooks overridden:** `OnCreate`, `OnUpdate`, `OnDelete`, `OnStep`, `GetNetBehavior`, `GetRelevantDistance`, `PreSendUpdate`, `PostReceiveUpdate`, `RenderRanges`, `OnShutDown`, `OnBoot`; new: `Play`, `SetState`, `OnPlayFadingSoundEnd`.

### 2. `CEnvSFXEntity`

Members (4): `en_hse` (the **sound environment** object, `CSoundEnvironment`, owned), `en_hre` (editor box, owned), `en_henParent` (null in all 136), `en_idParentPart`. Record (5): `ep_eEnvFX` "Environment" (`SfxEnvironment`, a preset, 18 distinct values among 0..19; most common 1 (22), 10 (18), 2 (16), 3 (14)), `ep_fSizeX`, `ep_fSizeY`, `ep_fSizeZ` "Width", "Height", "Depth" (7.8 to 3,743 m; **the edge lengths of the box**: the properties' bounding box is half the size on each side, R; `OnStretch` and `OnGetStretch` set and read the three), `ep_idParentPart`. **Behaviour (P/R):** `OnCreate` builds the sound environment from the record (preset and size) and the editor box (double sided); `OnBoot` attaches both to the parent part if there is a parent; `OnUpdate` re-reads the record (editor); `OnDelete` deletes the tree. `IsImportant` of the record is a constant stub. Not replicated, no step function. What the environment does to the sound (reverb of the listener inside the box) is the audio engine's; the 20 presets are open (**T01-180**).

### 3. `CFlareEntity`

Members (4): `en_hre` (the `CFlareRenderable`, owned), `en_henParent` (set 2,288: all on static models; null 213), `en_idParentPart` (0 in 2,491, one part in 10), `en_qvInitialPlacement`. Record (7; labels): `ep_pfcConfig` "Flare" (a `CFlareConfiguration`, set in 2,497, null in 4), `ep_colColor` "Color", `ep_fHotSpot` "Hotspot" (default 100; 0..250), `ep_fFallOff` "Falloff" (160; 10..1,200), `ep_fCoverage` "Coverage" (0.5; 0.1..3.18), `ep_fGlaring` "Glare" (0; 0..1), `ep_idParentPart`. **`OnCreate` (R) clamps the record**: hot spot ≥ 0, fall-off ≥ hot spot, coverage ≥ 0.01, glare to 0..1, and copies color, hot spot and fall-off to the renderable (script: all 2,501 records satisfy the four conditions). `OnBoot` clears the renderable's hidden flag, puts it at its initial placement and attaches it to the parent part (R); `OnShutDown` and `ExecuteDestruction` **set the renderable's hidden flag** (R), so a destroyed parent hides the flare; `GetCoverage` returns the renderable's coverage but at least 0.01 (R); the record's bounding box is a cube of half-size `GetCoverage` around the entity (R). `CollectForSortedIteration` forwards to the engine; `IsDestructible` is a constant stub. 132 virtual slots (27 beyond the base; the extra slots are shared empty stubs and `GetCoverage`, P). Not replicated.

### 4. `CMoviePlayerEntity` (binary-only)

`SMoviePlayerentity` (the DLL's spelling): `en_hre` (editor renderable, owned), `en_hmcMovieChannel` (the `CMovieChannel`), `en_pbmSource` (the movie), `en_ptxTarget` (target texture), `en_pmsTarget` (target sound), `en_bAutoPlay`. Record: "Source movie", "Target texture", "Target sound", "Autoplay" (default 1). `OnCreate` (P) gets the world's movie sync list, builds the channel and, if auto play, plays; `OnBoot` (P) initialises the channel (movie, texture, sound) and plays when auto play is on; macro functions `PlayOnce` (plays from 0), `Pause`, `Stop` (R); `OnDelete` deletes the channel. **No world places it and no code of the DLL creates it** (T01-181).

### 5. `CStaticVibrationEntity` (binary-only, no member table)

From the constructor and functions (P): handles to a `CVibroSource` and an editor renderable, a parent handle, the effect id, a start time, the parent part and an auto-play flag. `OnBoot` attaches to the parent and, **when auto play is on and an effect id is set, plays it looping**; `PlayOnce` plays the effect once, `PlayLooping` looping, `Stop` stops it; `OnShutDown` is `Stop`. The class has 19 virtual slots of its own beyond the base (14 shared stubs, P) and **no table member is registered**, so it cannot be saved or edited. No world has it.

## Invariants

1. 610 static sounds: record equal in all 610 for auto play, volume, pitch, random pitch, flags, delay and fall-off; state 0 in all; fades 0.
2. 266 of 610 auto-play; **a static sound is `NEB_STATIC` exactly when auto play is off** (344).
3. 136 environment sound effects: no parent, `SfxEnvironment` among 18 distinct values 0..19; box edges 7.8 to 3,743.
4. 2,501 flares: every record satisfies the clamp conditions; 2,288 are attached to a static model.
5. No movie player, vibration, simple, sound or spawn effect is in any world.

## Open questions

1. The static sound's sound flags (`SndFlags` bits: bit 0 looping; the others), the fades, `RenderRanges`, the editor cone: **T01-179**.
2. The `SfxEnvironment` presets 0..19 and what `CSoundEnvironment` does at run time: **T01-180**.
3. Who creates the movie player and the static vibration (editor catalogue, macro vocabulary) and their run-time use: **T01-181**.

## Test vectors

1. **Counts** (script): 610 static sounds (50 worlds), 136 environment effects (44), 2,501 flares (48).
2. **State.** Macro `PlayOnceWait` on a static sound with auto play off: state 2, fades clamped to the sound length, volume at time *t* of a sound of length 4 s with fades 1 s: `min(t / 1, (4 − t) / 1, 1)` times the volume (so 0.5 at t = 0.5 and at t = 3.5); with auto play on, a console warning and no state change.
3. **Auto play.** A static sound with auto play on and delay 2 starts at a random time in [0, 2) s after boot.
4. **Flare clamp.** A record with hot spot 100, fall-off 50 is changed to fall-off 100 at creation; coverage 0 becomes 0.01.
