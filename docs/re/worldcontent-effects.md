# Effect entities of `Sam2Game.dll`: particle, simple, sound, shaker, spawn and text effects (T01-110, part of T01-026)

- **Roadmap task:** T01-110 (overview [`worldcontent-classes.md`](worldcontent-classes.md); this page covers 13 exported classes plus the data's `CTextEffectEntity`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the identity of `CParticleEffectEntity` with the data's `CEffectEntity`, the member tables and their order, the counts, the copies from record to entity and the state values (`tools/re/worldcontent_classes_verify.py`), and, as of T01-178, for `CShakingParams`' member names (the existing generated schema, no new reading needed) and for `samCreateSchemeSound`'s mechanism, read in full; **medium** for the state machine, the net rules and the creation functions (one reading each, **nothing was run**), and for `samSpawnEffectForMaterial`'s two-tier dispatch (read in full, but three of its five or so named-event comparisons could not be matched to their literal text); **low** for the methods marked `P` or `N`, for the effect-life values, for `CTextEffectEntity` (confirmed to have no exported function at all, not merely unread) and for the remaining open questions.

Prerequisites: [`worldcontent-classes.md`](worldcontent-classes.md) (marks, order rule), `pfx.md`, `pfx-schema.md` and `effects-pipeline.md` (the particle effect resources, `CEffect`), `fx.md`, `wld-sound-effects.md` and `audio.md` (sounds), `replication.md`.

## Summary

| Class (export / data name) | In the data | What it is | Made by |
|---|---|---|---|
| `CParticleEffectEntity` / `CEffectEntity` | **2,352** entities (57 worlds), 2,347 records | a particle effect (`CEffect`, the `.pfx` resources) with an optional sound, a parent and a start/terminate/stop state machine | the editor; `samCreatePFX` (code) |
| `CParticleEffectSound` | 7-member structure inside the effect record | volume, pitch, random pitch, hot spot, fall-off, flags of the effect's sound | |
| `CShakerEffectEntity` | 12 entities (7 worlds), `SShakerEffectEntity` | a camera shake with an editor box | the editor; `samCreateShakerEffect` |
| `CSimpleEffectEntity` | not saved (DLL: `SSimpleEffectEntity`, 20 members) | a **fire-and-forget** effect plus sound at a place, with its own life time and net behaviour | code only: `samSpawnEffectForMaterial`, explosions, flames, damage, spawners, props touching fluid |
| `CSoundEffectEntity` | not saved (DLL: `SSoundEffectEntity`, 3 members) | a sound played once at a place | code only: `samCreateSchemeSound` |
| `CSpawnEffectEntity`, `CSpawnEffectCapsuleEntity` | not saved, no member table | **obsolete**: print `Class (%1) is obsolete and should be removed from level ASAP.` | nothing |
| `CTextEffectEntity` (data only, **no exported function**) | 14 entities (11 worlds), 14 records | a text drawn in the world (2D or 3D), see section 6 | the editor |

1. **The exported particle effect class is the data's `CEffectEntity`.** Same 18 members in the same order (script section 2), the same constructor defaults as the data (script section 3: `en_bEmitting` 1, `en_bNetEmitting` 1, the net state 0 in all 2,352 effects). The exported properties class is `CEffectProperties` (8 members). **High** (two readings plus the data).
2. **Started effects are the auto-start ones.** 2,028 of 2,352 effects have `en_bAutoStart` 1 and exactly 2,028 are in state `PES_STARTED` when saved; the other 324 are in `PES_NONE` (state 0) and wait for a macro (`Start`, `Terminate`, `Stop`) (script).
3. **Two effect classes are code-made.** The simple effect and the sound effect have no record in any world; the game creates them with a properties object that exists only in memory (section 3, 4).

## Evidence

- **Export table.** 13 classes, 145 exported functions (55 named): particle effect 33 + 11 + 6, simple effect 13 + 7, sound effect 12 + 9, shaker 15 + 9, spawn effects 8 + 7 + 8 + 7.
- **Member tables.** `CEffectEntity` (18), `CEffectProperties` (8), `CParticleEffectSound` (7), `SShakerEffectEntity` (3) and `...Properties` (2), `CTextEffectEntity` (5) and `...Properties` (10) are saved and match the data; `SSimpleEffectEntity` (20), `SSimpleEffectProperties` (16), `SSoundEffectEntity` (3), `SSoundEffectProperties` (10) are declared but not saved. The shaker structure `CShakingParams` is a 10-word structure (not expanded here).
- **Identity of `CParticleEffectEntity` and `CEffectEntity`** (two readings): the member table registered as `CEffectEntity` has 18 members with the **same layout** as the constructor and the net functions of `CParticleEffectEntity` (the constructor stores 1 into auto start, emitting and net emitting, and 0 into the state, the lifetime type, the pitch members and the handles; `PreSendUpdate` copies the state and the emitting flag into the two flagged members), and the data's entities carry those values.
- **Methods.** Read: the constructors; the particle effect's `OnBoot`, `OnShutDown`, `SetState`, `Start`, `Stop`, `Terminate`, `StateStarted`, `StateTerminating`, `StateStopped`, `EnableEmitting`, `DisableEmitting`, `PostReceiveUpdate`, `PreSendUpdate`, `GetNetBehavior`, `PerformParenting`, `OnDelete`; the simple effect's `GetNetBehavior`, `OnBoot`, `OnDelete`, `GetRelevantDistance`; the shaker's `OnCreate`. Profiled: `OnCreate` of the particle, simple and sound effects, `OnUpdate`, the properties' `CreateEffectEntity` and `GetBoundingBox`.
- **Creators** were found as callers of the properties classes' constructors (names as exported or as named in the DLL).
- **Not checked:** nothing was run; the values of `EffectLifeType`, the sound flags (`SndFlags`) and the particle effect resources were not read; `CTextEffectEntity` has no exported function, so its behaviour was not read.
- **T01-178** additionally read, in full, `Sam2Game.dll`'s `samSpawnEffectForMaterial` and `samCreateSchemeSound`; `CShakingParams`' member names were taken from the existing `wld-entity-schema.md` (generated from the data's own type table, no new binary reading). `samCreateShakerEffect` and the material's own per-event lookup helpers were not read.

## Layout / rules

### 1. The particle effect (`CParticleEffectEntity`, data `CEffectEntity`)

**Members** (18; **R** replicated, `@` owned).

| # | Binary name | Meaning | Data (2,352) |
|---|---|---|---|
| 5 `@` | `en_hedr` | editor-only renderable (a small primitive) | set |
| 4 `@` | `en_hre` | the effect renderable (`CEffectRenderable`) | set |
| 6 | `en_fxltLifeType` | `EffectLifeType` of the effect (taken from the effect resource at creation) | 0 in 123, 1 in 2,192, 2 in 37 |
| 7 | `en_bSpawnedEffect` | made by code (deleted at shut-down, not hidden) | 1 in **5**, 0 in 2,347 (the 5 are the entities without a record, `wld-entities.md`) |
| named | `en_bAutoStart` | start at boot | 1 in 2,028; equals the record in all 2,347 |
| named | `en_idParentPart`, `en_idParentPartAttachment` | the parent's part and attachment | equal the record in all |
| named | `en_henParent` | the entity it is attached to | set 1,675: static models 1,644, animated movers 29, a character 1, a prop 1; null 677 |
| named `@` | `en_hss` | the sound source | set |
| named | `en_ulSndFlags`, `en_pbsSound`, `en_fPitch`, `en_fPitchRnd` | the sound (copied from the record's `ep_pesSound`) | sound set in 1,184; flags 3 (1,243), 2 (1,088), 7 (21); pitch 1 in 2,274 |
| named | `en_qvOrgPlacement` | the placement at creation | structure |
| named | `en_pesEffectState` | `ParticleEffectState`: `PES_NONE` 0, `PES_STARTED` 1, `PES_TERMINATING` 2, `PES_STOPPED` 3 | 1 in 2,028, 0 in 324 |
| named **R** | `en_pesNetEffectState` | the net copy of the state | 0 in all |
| named | `en_bEmitting` | the effect emits particles | 1 in all |
| named **R** | `en_bNetEmitting` | the net copy | 1 in all |

**Record** (`CEffectProperties`, 8; labels from the DLL): `ep_pefEffect` "Effect" (a `CEffect`, set in all 2,347), `ep_pesSound` "Sound" (a `CParticleEffectSound`: "Sound", "Volume" 0.35..1, "Pitch" 0.5..1.25, "Random pitch" 0.01..0.2, "Hot spot" 1..100, "Fall off" 5..300, "Flags" 2, 3 or 7; constructor defaults 1, 1, 0.01, 5, 10, 2), `ep_fStretch` "Stretch" (default 1; −1.7..20 in the data), `ep_idParentPart` "Parent part", `ep_idParentPartAttachment` "Parent part attachment", `ep_bSpawnedEffect` (0 in all records), `ep_bAutoStart` "Auto start" (default 1), `ep_hepRelevantEntity` "Relevant entity" (2 set): the entity whose visibility the effect follows. `OnStretch` sets the stretch; `OnGetStretch` returns it on all three axes (R).

**State machine (R).** `SetState(s)` stores the state and runs `StateStarted` (1), `StateTerminating` (2) or `StateStopped` (3).
- `OnBoot`: place the effect at its stored placement; if there is a parent entity, attach to the parent's part and attachment (`PerformParenting`); if auto start is set, state 1.
- **Started:** start the effect renderable, set emitting, make it visible; for life type 0 read the effect's existence time; set the sound's pitch and play the sound (looping per bit 0 of the sound flags); **when the entity is a spawned effect, its life type is 0 and its sound does not loop, schedule `Terminate` after the longer of the effect's existence time and the sound's length** (so an editor-placed or auto-started effect never ends by itself).
- **Terminating:** disable emitting (the particles die out) and stop the sound; a spawned effect without a renderable is deleted at once, otherwise it schedules its own end after the effect's dying time.
- **Stopped:** stop the effect and the sound immediately.
- Macro functions `Start`, `Terminate`, `Stop` set the state; **`Start`, `Terminate`, `Stop`, `EnableEmitting` and `DisableEmitting` print an error and do nothing when `en_bAutoStart` is on** (`Can't execute macro function "Start()" on effect entity which has enabled auto start flag.`). `ClearEmittedParticles` removes the live particles. `EnableEmitting` and `DisableEmitting` set `en_bEmitting` and start or stop the sound.
- `OnShutDown`: a spawned effect is deleted; otherwise state 3 and the renderable is hidden. `OnDelete` deletes the three owned objects (renderable, editor renderable, sound source).
- `PostReceiveUpdate` (client): when the net state differs from the state, apply it (the same three functions); when the net emitting flag differs, enable or disable emitting.
- `SetEffectPlacementProvider(p)` hands the renderable an object that gives the effect its placement each frame (used by weapons and the body; P). `GetRelevantDistance` is the effect's own range member, else the large constant.

**Net.** Overview section 4: static when the parent is destructible or when not auto-started; else none; 2 of 18 members replicated. `CParticleEffectProperties::CreateEffectEntity(effect, factor)` (R) is the code path: it sets the record's effect, **marks it a spawned effect, sets the stretch to `factor` times the effect's own stretch** and spawns the entity; `samCreatePFX` is its caller. **Hooks overridden:** `OnCreate`, `OnUpdate`, `OnDelete`, `OnBoot`, `OnShutDown`, `SetAbsPlacement`, `GetAbsPlacement`, `GetBoundingBox`, `CollectForSortedIteration`, `GetNetBehavior`, `GetRelevantDistance`, `PreSendUpdate`, `PostReceiveUpdate`, `ExecuteDestruction` (sets the renderable's hidden flag; profiled only, T01-177); new: `PerformParenting`, `SetEffectPlacementProvider`.

### 2. `CParticleEffectSound` (value structure, 7 members)

`pes_pbsSound` (the sound resource), `pes_fVolume`, `pes_fPitch`, `pes_fRndPitch`, `pes_fHotSpot`, `pes_fFallOff`, `pes_ulFlags` (`SndFlags`). It is in 57 world files and one `.rsc`. It has no method besides the standard ones.

**The material effect table (T01-178: `samSpawnEffectForMaterial`, read in full).** The function takes an event `IDENT` (for example `BulletHit`) and a `CMaterialAttributes*`; if the material is null it does nothing. It then:

1. Looks the event up **in the material's own per-event weighted effect/sound table** (an unnamed helper that, by its signature, is the material's own "get the effect for this event" lookup) — this is the "material effect table" itself: every `CMaterialAttributes` carries one, keyed by the event name, and that lookup is what actually chooses which effect and sound a given surface plays for a given event.
2. If the material has nothing for that event, no effect is spawned.
3. Otherwise, when the caller asks for it (one boolean parameter), the function **overrides the hot spot and fall-off** with values read straight from the global game parameters, but only for a short, hardcoded list of event names it compares the event `IDENT` against one at a time: **confirmed** `Thrust`, `ChainSaw` and `BeamHit` (two more comparisons in the chain could not be pinned to their literal text in this reading); for any other event name, when that override flag is set, the function spawns **nothing**. When the override flag is not set, the hot spot and fall-off are the plain defaults **5.0** and **20.0**.
4. The chosen effect and sound, the placement, and the remaining caller-supplied numbers (volume, pitch, stretch, …) are packed into a `CSimpleEffectEntity` and spawned the usual way (section 3).

So the "table" the open question asked about is genuinely **two tables**: the material's own per-event effect/sound entries (not expanded further here — the helper itself was not traced into `CMaterialAttributes`'s own layout), and a short, separate, hardcoded list of named events that bypass the material's hot spot/fall-off in favour of global constants.

**`samCreateSchemeSound`, read in full (T01-178).** Picks a weighted sound from the **model's own sound scheme** (`samGetSchemeSound(modelConfig, schemeId)` — the same `CSoundScheme` `CSamCustomModelData` names, `mdl.md` T01-057) by a given scheme `IDENT`, builds a `CSoundEffectProperties` from it (the sound resource, the scheme entry's own volume and flags, a randomised pitch) and spawns the `CSoundEffectEntity`; returns nothing when the model's scheme has no sound for that id.

**`CShakingParams` (10 words), named in full from the existing schema (T01-178; no new binary reading needed — `wld-entity-schema.md` already has the names from the data's own type table):** `sp_fHotSpot`, `sp_fFallOff` (the same distance-falloff shape as a sound), `sp_fDamping`, `sp_fYIntensity`, `sp_fZIntensity`, `sp_fBIntensity` (bank/roll), `sp_fYFreq`, `sp_fZFreq`, `sp_fBFreq` (one frequency per intensity axis) and `sp_fConstIntensityPeriod` (an initial period, presumably, during which the shake does not decay).

**`CTextEffectEntity` and `samCreateShakerEffect`: still open.** The text effect genuinely has no exported function in this build (confirmed again in this reading, not just assumed); its behaviour is entirely the engine's own text-rendering system and cannot be read from `Sam2Game.dll`. `samCreateShakerEffect` and `FUN_10227b90`/`FUN_10227cc0` (the material's own lookup helpers) were not read this task.

### 3. The simple effect (`CSimpleEffectEntity`, binary-only)

**Role.** A one-shot effect and sound at a place, created by code and deleted when its life time ends. Members (`SSimpleEffectEntity`, 20): `en_hre`, `en_hss` (renderable and sound source, owned), `en_fLifeTime` (default 1e-5 s, then raised to the effect's existence time or the sound length), `en_nebNetBehavior` (default 0), and **16 replicated** members: `en_ulSoundFlags` (2), `en_pbsSound`, `en_pfxEffect`, `en_qvInitialPlacement`, `en_fEffectStretch` (1), `en_fEffectPower` (1), `en_vCollisionNormal`, `en_vCollisionPenetration`, `en_fVolume` (1), `en_fPitch` (1), `en_fPitchRnd` (0), `en_fHotSpot` (10), `en_fFallOff` (100), `en_iConeAngle` (360), `en_iSoftAngle` (360), `en_fConeVolume` (0) (defaults from the constructor, **medium**). The record `SSimpleEffectProperties` (16) holds the same settings plus `ep_nebNetBehavior`. **Behaviour (R/P):** `OnCreate` builds the effect renderable, starts it, takes the dying time, sets up the sound source and schedules its own end after the longer of life time and effect time (P, 2,120 instructions); `OnBoot` plays the sound (R); `OnDelete` deletes the renderable tree and the sound source; `OnShutDown` deletes the entity (R); `GetNetBehavior` returns `en_nebNetBehavior` (R); `GetRelevantDistance` is the effect's range member (R). **Creators** (callers of the properties constructor): material hit effects (`samSpawnEffectForMaterial`), explosions (`SpawnExplosion`), flames (`AddFlame`), damage (`DoDamage`), `DoSpawnEffect` and `SpawnOneSpawnee` (the spawner's effect), and the prop's fluid contact (`CPropEntity::OnFluidTouched`).

### 4. The sound effect (`CSoundEffectEntity`, binary-only)

Members (`SSoundEffectEntity`, 3): `en_hss` (sound source), `en_pbs` (sound), `en_ulSoundFlags` (all 0 at construction). Record (`SSoundEffectProperties`, 10; labels "Sound", "Volume", "Pitch", "Random pitch", "Hot Spot", "Fall Off", "Cone angle", "Soft angle", "Outside cone volume", "Flags"; defaults volume 1, pitch 1, random pitch 0.05, hot spot 10, fall-off 100, cones 360, flags 2). `OnBoot` plays the sound and, unless the flags say it loops, schedules the entity's end after the sound's length (R); `OnShutDown` deletes it; `CreateEffectEntity` (P) builds the properties from a weighted sound and spawns it; created by `samCreateSchemeSound`. Hooks: `OnCreate`, `OnDelete`, `OnBoot`; 111 slots (6 new, P).

### 5. The shaker (`CShakerEffectEntity`, data `SShakerEffectEntity`)

12 placed in 7 worlds, **all with `en_bCreatedInEditor` 1** (a flag copied from the record, default 1). Members: `en_hreShaker` (the shaker renderable), `en_hreEditorRenderable` (a box, drawn in the editor), `en_bCreatedInEditor`. Record: `ep_spShakingParams` "Shaking parameters" (a 10-word `CShakingParams`), `ep_bCreatedInEditor`. **Behaviour (R/P):** `OnCreate` builds both renderables, copies the 10 shaking words to the shaker, and **if the entity was not created in the editor it starts shaking at once** and schedules its own end after the shaker's dying time; an editor-placed shaker waits for `StartShaking` (the macro or an animation event); `OnShutDown` stops shaking and deletes; `RenderRanges` draws the shaking range in the editor (P); `CreateEffectEntity` is the code path (`samCreateShakerEffect`; P). `OnStart` is an empty hook. Hooks: `OnCreate`, `OnDelete`, `OnShutDown`, `GetAbsPlacement`, `RenderRanges`, `OnStart` (new).

### 6. The obsolete spawn effects and the text effect

- `CSpawnEffectEntity` and `CSpawnEffectCapsuleEntity` (and their properties): the default constructor only builds the base; `OnCreate` is the obsolete message (the same function as the obsolete brains, puppets and items). Not saved in any world (script). The spawner's `ep_sfcSpawnEffectConfig` (a `CSpawnEffectConfiguration` resource, 11 `.sfc` files) is **not** an entity: it is read by the spawner (`T01-109`).
- `CTextEffectEntity` (data base `CBaseEntity`, 5 members): `en_ptrfxRenderingEffect2D`, `en_ptrfxRenderingEffect3D` (owned), `en_pre` (renderable, owned), `en_ptfxConfiguration`, `en_ptfxbcBackgroundConfig`. Record (10): "Effect configuration", "Effect background configuration", "Text" (a translatable string), "Type" (`TextRenderingEffectTypes`, 0 in all 14), "3D text width" (1 in 12, 3 in 2), "Face forward" (0), "Double sided" (1 in all), "Stay forever" (0 in 12, 1 in 2), "Render text region" (0), "Parent part". The 2D effect handle is set in all 14, the 3D one is null (the type 0 is 2D); the macro calls that drive it are RPCs (`replication.md`). The DLL exports no function of these two classes: their behaviour is the engine's text effect system (**T01-178**).

## Invariants

1. 2,352 `CEffectEntity` in 57 worlds: 2,347 with a record and 5 spawned effects without one (`en_bSpawnedEffect` 1 in exactly those 5).
2. For all 2,347 pairs `en_bAutoStart`, `en_idParentPart`, `en_idParentPartAttachment` and `en_bSpawnedEffect` equal the record's.
3. State: 2,028 effects are `PES_STARTED` and have auto start; 324 are `PES_NONE`; no saved effect is in state 2 or 3.
4. `en_bEmitting` and `en_bNetEmitting` are 1 and `en_pesNetEffectState` is 0 in every saved effect.
5. No world contains a simple effect, a sound effect, a spawn effect or a spawn-effect capsule (script).
6. The 12 shakers are all editor-created.

## Open questions

1. The remaining particle-effect behaviour: the `EffectLifeType` values (0, 1, 2) and which effect resources give which, `OnCreate`, the effect placement provider, the relevant entity, `ExecuteDestruction`: **T01-177**.
2. ~~The creation functions of the simple and sound effects.~~ **Answered (T01-178)**, see section 3 above: `samSpawnEffectForMaterial`'s dispatch is two-tier (the material's own per-event table, plus a short hardcoded list of named events — `Thrust`, `ChainSaw`, `BeamHit` confirmed, two more not pinned down — that override the hot spot/fall-off from global parameters); `samCreateSchemeSound` picks a weighted sound from the model's own sound scheme. **The text effect entity stays open**: confirmed (not merely unread) to have no exported function in this build at all; its behaviour is the engine's own text-rendering system.
3. ~~Shaking parameters (`CShakingParams` 10 words).~~ **Answered (T01-178)**, from the existing generated schema, no new reading needed: `sp_fHotSpot`, `sp_fFallOff`, `sp_fDamping`, `sp_fYIntensity`, `sp_fZIntensity`, `sp_fBIntensity`, `sp_fYFreq`, `sp_fZFreq`, `sp_fBFreq`, `sp_fConstIntensityPeriod`. The shaker's range display (`RenderRanges`) and `samCreateShakerEffect` itself stay open, not read this task.

## Test vectors

1. **Counts** (script): 2,352 particle effects in 57 worlds (2,347 records), 12 shakers in 7 worlds, 14 text effects in 11 worlds.
2. **States.** A saved effect with auto start 1 has `en_pesEffectState` 1; one with auto start 0 has 0 and is started by the macro `Start` (an error is printed when auto start is on).
3. **Life of a spawned effect.** `CreateEffectEntity(effect, 2.0)` on an effect whose own stretch is 0.5 spawns an effect entity with `en_bSpawnedEffect` 1 and stretch 1.0; for life type 0 and a non-looping sound it is terminated after max(existence time, sound length); then its particles stop emitting and it ends after the effect's dying time.
4. **Net.** A particle effect attached to a static model that has a destruction holder is `NEB_STATIC`; one that is auto-started and has a non-destructible or no parent is `NEB_NONE`.
