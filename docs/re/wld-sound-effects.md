# `.wld` sound and effects: sources, channels, environments, effect instances (T01-012h)

- **Roadmap task:** T01-012h (part h of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for how sound sources and channels are stored and who holds them, the sync list, feeders, environment boxes, and for how effect entities, effect renderables and effect instances relate to the `.pfx` they run (all checked on all 59 worlds with `tools/re/wld_sound_verify.py`; members come from the files' own type tables, see `wld-sound-effects-schema.md`); **medium** for the roles of the channel members (names and value ranges; `audio.md` fixes the two radii); **low** for run-time behaviour (when a slot gets a sound, the attenuation curve, how effect states change). Scope: all 59 distinct worlds.

Prerequisites: `audio.md` (the sound system, ranges, music, environments: it describes the same objects from 5 sample worlds; this spec is the full-corpus layout), `pfx.md` (the effects the instances run), `wld-root.md`, `wld-entities.md`, `wld-lighting.md` (haze, bloom, flares).

## Summary

- **A sound slot is a source plus a channel.** The world holds 71,991 `CSoundSource` (a placed aspect) and 71,991 `CSoundChannel` (the voice's parameters), one channel per source. **Only 1,362 channels (1.9 %) play a sound**; 70,629 are empty slots that entities reserve and fill at run time (a static model reserves two, a legged character up to seven, a door two).
- **Sounds are `.wav` files.** Every channel that has a sound (1,362 of 1,362) names an external `.wav` and has a **feeder** (`CStaticSoundFeeder`); the world's **sync list** is the hub: it lists exactly those channels, and the sound environments.
- **Effects are `.pfx` instances.** 2,352 `CEffectEntity` each own an effect renderable and a sound source. An effect renderable holds a `CEffectInstance` naming the `.pfx` and one **particle effect instance per particle effect** of that file. 3,590 effect renderables in all, 1,238 of them owned by pickups (the glow of items).
- Reverb zones are `CEnvSFXEntity` (136) with a `CSoundEnvironment` box each (preset values 0 to 19).

## Evidence

`tools/re/wld_sound_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes) over the retail install with all patches, 59 worlds:

- **Sources and channels:** every source's `CSoundSource.1` is a channel of the world and **no channel is shared** (71,991 of 71,991); 71,819 sources are held by an entity member (table below), 172 are not; 708 sources have no owner aspect.
- **Channels with a sound** (`CSoundChannel.3`): 1,362, all external `.wav`; each has a feeder (`.2`), and 1,362 feeders exist.
- **Sync list:** one per world (59). Its channel list `.3` equals the set of channels that have a sound and its environment list `.7` equals the world's `CSoundEnvironment` objects in every world; the listener array `.4` is empty in every world; 71,283 channels name the list (`CSoundChannel.4`).
- **Effects:** 2,352 effect entities, each with a renderable that names it back (`CAspect.5`) and a sound source (`en_hss`); 3,537 of the 3,590 effect instances name an external `.pfx` and all of them have **as many particle effect instances as the `.pfx` has particle effects, of the same classes in the same order** (3,537 of 3,537); the entity's `EffectLifeType` (`.6`) equals the file's `CEffect.12` in 2,299 of 2,299 entities that name one.

## Sources and channels

```
CSoundSource  (derives CAspect; a placed aspect)
  CSoundSource.1  -> its CSoundChannel
CSoundChannel
  .1  SndFlags   0 (70,629 channels), 3 (1,212), 7 (143), 1 (4), 2 (2), 5 (1): all non-zero values are on channels that have a sound
  .3  -> sound   an external .wav (1,362) or null            .2 -> feeder (a CStaticSoundFeeder) exactly when .3 is set
  .4  -> the world's CSoundSyncList (71,283 channels)
  .10 volume     with a sound: 1.0 (582), 0.6 (484), 0.5 (187), 0.75 (70), others 0.1 to 0.85
  .11 pitch      .20 hot spot: 2 (813), 1 (287), 5 (45), 10, 20, 50, 500, ...     .21 fall-off: 10 (842), 15 (148), 5 (41), 150, 8, 35, ... up to 700
  .22 flag       1 (1,343) or 0 (19) among channels with a sound
  .33 .34        cone angles in degrees: 360 and 360 (1,358), 120 (3), 90 (1)
  .25 QuatVect, .26 Vector3f   placement and velocity of the voice (zero in the stored state)
```

- **Which entity holds a source** (each slot is a handle member of the entity): `CStaticModelEntity.en_hssAlive` and `.en_hssDestruction` (25,383 each: 50,766 slots), `SPuppetEntity.en_hssMouth`, `en_hssMoving`, `en_hssAlive`, `en_hss0` to `en_hss3` (characters: 11,869 of the sources belong to legged characters alone), `SPropEntity.en_hssDestruction` (1,726), `SDoorEntity.en_hssAnimEvents` and `.en_hssMoving` (352 sources), `CEffectEntity.en_hss` (2,352), `SStaticSoundEntity.1` (610), `SMissionObjectiveEntity.en_hss` (131) and `CSecretEntity.en_hss` (110). Source owners by class (`CAspect.5`): static models 50,938, legged characters 11,869, effects 2,352, props 1,726, aircraft 1,616, turrets 984, static sound entities 610, doors 352, rolling-ball characters 308, hovercraft 287, objectives 131, secrets 110, none 708.
- **Reading.** A source with an empty channel is a **reserved voice**: the entity plays a sound on it when the event comes (alive loop, destruction, mouth, moving); the data does not say which sound. Only the **static sound entities, effect entities with their own sound and a few others** carry a sound in the stored state.
- **Ranges.** The hot spot and fall-off are the two radii of `audio.md` (full volume inside the hot spot, silent beyond the fall-off); the curve between is not stored.

## Static sounds, environments, the sync list

- **`SStaticSoundEntity`** (610 in 50 worlds; record `SStaticSoundProperties`): `.1` → its source, `.3` the sound (an external `.wav`), `.4` `SndFlags` (0 to 8), floats for volume (0.1 to 1), pitch (0.25 to 1.5), pitch randomness (0 to 0.5), hot spot and fall-off (4 to 100,001), cone angles 120, 360 or 90 (in the record), a `StaticSoundState` (0 in the stored state), a `CMacroLatentID` (`wld-macro.md`) and `.15` → an optional entity (86 set). It is the editor's ambient sound; the roles of the individual floats are in `audio.md` and the schema.
- **`CSoundSyncList`** (59): `.3` the channels that play a sound, `.7` the environments, `.4` the listeners (empty: listeners exist only at run time), `.5` a `Time`, `.6` 1, `.8` 0.
- **`CSoundEnvironment`** (136 in 44 worlds): `.1` a `Box3f`, `.2` a **`SfxEnvironment`** preset (values 1: 22 boxes, 10: 18, 2: 16, 3: 14, 5: 12, 14: 7, 9: 6, 11: 6, 4: 6, 6: 5, 8: 5, 7: 4, 0: 3, 15: 3 among the 14 most common; the range is 0 to 19), `.3` → the sync list. Each is owned by a **`CEnvSFXEntity`** (136; its record `CEnvSFXProperties` holds the preset and three floats of 7.8 to 3,744: probably the box size).
- **Music.** `CDynamicMusic` and its two `CDynamicMusicSource` per world are described in `audio.md`; their sound sources are held through the world info entity (the nested `CDynamicMusicSource.dms_pssSource`).

## Effects

```
CEffectEntity (2,352 in 57 worlds; record CEffectProperties, 2,347)
  .4  -> its CEffectRenderable (names the entity back)       .5 -> editor renderable
  .6  EffectLifeType 1 (2,192), 0 (123), 2 (37) = the .pfx CEffect.12
  en_bAutoStart / en_bEmitting / en_pesEffectState: (1, 1, state 1) 2,028 entities, (0, 1, state 0) 324 entities
  en_hss -> its sound source;   en_pbsSound -> its own sound (set in 1,184), en_fPitch, en_fPitchRnd, en_henParent, en_idParentPart
CEffectRenderable (3,590; owners: 2,352 effect entities and 1,238 pickups)
  CEffectRenderable.1  CEffectInstance:  .7 -> the effect (an external .pfx; null in 53), .6 list of particle effect instances (1 to 12),
                       .1 .3 Time, .4 QuatVect, .11 .12 .13 Vector3f, .8 FLOAT (-1.7 to 20), .9 1, .10/.2 flags, .15 -> an entity (2)
  CEffectRenderable.2  Box3f bounds,  .3 IDENT attachment (Top, Engine, ...),  .4 null
CMachineParticleEffectInstance / CCloudsParticleEffectInstance / CAtomicParticleEffectInstance (derive CParticleEffectInstance)
  .1 QuatVect placement; the machine instance adds a Time, an INDEX 0 and an empty list: run-time state, empty in the stored state
```

- **One instance per particle effect, in the file's order**, with the matching class (`CMachineParticleEffectInstance` for `CMachineParticleEffect`, and so on): the runtime builds the instance list from the `.pfx` (`pfx.md`); the stored instances carry no simulation state.
- **Pickups** (items) have an effect renderable each (an effect on the pickup, for example `GreenFlare.pfx`): `SShellsItemEntity` 143, `SHealthItemXSEntity` 119, `SHealthItemMEntity` 105, … The effects used most by effect entities are small fires (`fire_small` 955 entities), fireflies (`firefly_white` 215), temple fires, lava smoke and ventilator smoke.
- **`CEffectProperties`**: `.1` the `.pfx`, `.2` FLOAT, `.5` and `.6` part and attachment names (`Model`, `Body`, `Top`, `Engine`), `.8` BOOL, `.10` a **`CParticleEffectSound`** (`pes_pbsSound` set in 1,184 of 2,347, volume 0.35 to 1, pitch 0.5 to 1.25, random pitch 0.01 to 0.2, hot spot 1 to 100, fall-off 5 to 300, flags 2, 3 or 7): the effect's own sound.
- **Other effect entities.** `SShakerEffectEntity` (12 in 7 worlds, with a `CShakerRenderable` holding `CShakingParams`: shaking parameters), `CTextEffectEntity` (14 in 11 worlds: a text effect with a `CTextEffectConfiguration`, the `.tfx` of `fx.md`), `CHazeEffectRenderable` (65), `CBloomRenderable` (`wld-lighting.md`).

## Reader invariants

1. A world's sound slots are source plus channel pairs; most are empty: treat an empty channel as "no sound yet", not as an error. A channel with a sound has an external `.wav` and a feeder.
2. Find a slot through the entity member that holds it (list above); the channel's `CSoundChannel.4` names the world's sync list.
3. The sync list needs no reconstruction: it is exactly the channels with a sound and the environments.
4. An effect entity runs the `.pfx` of its renderable's instance; build the particle instances from the `.pfx` (one per particle effect, same classes, same order); the stored instance state is empty.
5. `en_bAutoStart` 0 entities (324) are not started by the world; whether `en_bEmitting` 1 matters at that point is not stated by the data.
6. Unknown sound or effect classes must fail loudly; the classes in `wld-sound-effects-schema.md` are the complete list of the retail data.

## Test vector

`Content/SeriousSam2/Levels/Multiplayer/DM_Branchester/DM_Branchester.wld` (4.0 MB): 307 sound sources and 307 channels (all with `SndFlags` 0 and no sound), one `CSoundSyncList` with empty lists; 11 `CEffectEntity` and 67 `CEffectRenderable` (the rest owned by pickups); the effect entity 10130 (`en_bAutoStart` 1, `EffectLifeType` 1, parent a `CStaticModelEntity`) owns an effect renderable and the sound source 709; a pickup's renderable (`SSniperItemEntity`, owner of an `CEffectRenderable` with `CAspect.6` `0x400005`) holds a `CEffectInstance` naming `GreenFlare.pfx` with one `CMachineParticleEffectInstance`.

## Open questions

- **Run-time behaviour:** which sound fills a reserved slot, how the attenuation curve runs between hot spot and fall-off, what `SndFlags` bits mean (`mat.md`), and what an effect's `en_pesEffectState` and `en_bEmitting` do (T01-034 continues in the game rules, T04 renderer for effects).
- The 172 sources held by no entity member and the 708 without an owner aspect, the 53 effect instances without a `.pfx`, `CEffectInstance.2` (a BOOL member with values up to 2e9: stale memory?), `.8`, `.10`, and `CEffectRenderable.3`.
- The meaning of `CSoundSyncList.5`/`.6`/`.8` and of `SStaticSoundProperties` members that have only numbers.
