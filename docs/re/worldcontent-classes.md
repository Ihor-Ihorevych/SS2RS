# World content entity classes of `Sam2Game.dll`: overview and the base class (T01-110, part of T01-026)

- **Roadmap task:** T01-110 (part 9 of the class inventory T01-026; the other parts are T01-102 to T01-113). Companion pages: [`worldcontent-models-props.md`](worldcontent-models-props.md) (static models and props), [`worldcontent-effects.md`](worldcontent-effects.md) (particle, simple, sound, shaker, spawn and text effects), [`worldcontent-sound-media.md`](worldcontent-sound-media.md) (static sounds, static vibration, environment sound effects, movie players, flares) and [`worldcontent-markers-debug.md`](worldcontent-markers-debug.md) (path, visibility-box and navigation-mesh markers, the debug view helpers).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/worldcontent_classes_verify.py` reproduces from the data (which of the 55 classes are saved and under which name, the member lists, the member-order rule that names every numbered member of 28 data classes, the entity and record counts, the copies from record to entity, the flag values) and for the class defaults that two independent readings give (the constructor against the value stored in every placed entity); **medium** for everything read from the binary once (the virtual hooks, the net behaviour, the creation functions, the state machines, the macro functions); **low** for what is described from a name only (marked `N`) and for the questions under "Open questions". Nothing was run.

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `wld-entities.md` and `wld-entity-schema.md` (the entities and records of the worlds), `wld-geometry.md` (static models as level geometry), `wld-visibility.md`, `wld-navigation.md`, `wld-sound-effects.md`, `pfx.md` and `effects-pipeline.md` (what the data holds for the things these classes drive), `replication.md` (net behaviour, replicated members), `item-classes.md` (the neighbouring class family and the member-order rule).

## Summary

`Sam2Game.dll` exports **55 classes** for the content a level designer places in a world and that is not a character, an item, a weapon or a mechanism (854 exported functions, 471 of them named methods besides the 7-symbol standard set): **18 entity classes**, **18 properties classes**, one value structure (`CParticleEffectSound`) and **18 debug helper classes**. In the data, **25** of the 55 are saved (23 under the exported name or its `S` twin, 2 more under the name the DLL registers them with); **30** are not saved: the 18 debug helpers and 6 entity and 6 properties classes that no world file contains.

| Part | Classes (entity and properties each, unless said) | Page | Classes | Symbols (named) |
|---|---|---|---|---|
| base | `CBaseEntity`, `SBaseEntity`, `CBaseProperties`, `SBaseProperties` | this page | 4 | 169 (141) |
| static models and props | `CStaticModelEntity`, `CPropEntity` | [`worldcontent-models-props.md`](worldcontent-models-props.md) | 4 | 175 (147) |
| effects | `CParticleEffectEntity` (saved as `CEffectEntity`), `CSimpleEffectEntity`, `CSoundEffectEntity`, `CShakerEffectEntity`, `CSpawnEffectEntity`, `CSpawnEffectCapsuleEntity`, the structure `CParticleEffectSound` | [`worldcontent-effects.md`](worldcontent-effects.md) | 13 | 145 (55) |
| sound, vibration, movies, flares | `CStaticSoundEntity`, `CStaticVibrationEntity`, `CEnvSFXEntity`, `CMoviePlayerEntity`, `CFlareEntity` | [`worldcontent-sound-media.md`](worldcontent-sound-media.md) | 10 | 135 (65) |
| markers | `CPathMarkerEntity`, `CVisBoxEntity`, `CNavMeshMarkerEntity` | [`worldcontent-markers-debug.md`](worldcontent-markers-debug.md) | 6 | 69 (27) |
| debug helpers | `CDebugObject`, `CDebugDumpConfig` and 16 dump and view classes | [`worldcontent-markers-debug.md`](worldcontent-markers-debug.md) | 18 | 161 (36) |

What a reader needs to know first:

1. **Names.** The data stores 11 of the exported classes under the exported `C` name and 12 as the `S` twin (`CPropEntity` is exported, `SPropEntity` is saved), the same rule as the other parts. **Two exported classes are saved under a different name:** `CParticleEffectEntity` and `CParticleEffectProperties` are the classes the data calls `CEffectEntity` and `CEffectProperties` (the DLL registers the particle effect under that name, the member table and the constructor agree on every member; 2,352 placed effects, section 1). Two further data classes of the same part have no export: the text effect `CTextEffectEntity` and `CTextEffectProperties` (14 placed); the engine bases `CEntity` and `CEntityProperties` are listed with them in section 1.
2. **Every class of the family is a thin layer.** The entity is the run-time copy of an editable *record* (`...Properties`); `OnCreate` copies the record into `en_` members, builds one or two *renderables* (the visible or audible thing and an editor-only box or arrow), and the later hooks only toggle them. The record's `ep_` members and the entity's `en_` members carry the same names in the same order wherever they mean the same thing, so a numbered data member is named by position (section 3, the **member-order rule**, **high** for 28 data classes).
3. **The data is a snapshot of construction state.** Every placed entity stores the constructor's default for the members that no creation function overwrites (animation fade 0.2 s, loop count −1, speed and strength 1, accumulated damage 0, emitting 1, ...): the script checks every one of them against every placed entity (**high**, two readings: the constructor and the data).
4. **Net behaviour is decided per class and is not what the names suggest** (section 4): a static model is *static* (replicated) only when its model can be destroyed or when it is flagged "Control by macro"; a particle effect is static when its parent can be destroyed or when it is not auto-started; a prop is dynamic unless a spawner made it; a static sound is static unless it auto-plays; a simple effect carries its own choice; the other 11 entity classes are never replicated. This resolves three items that `replication.md` left open (the 15 replicated members of `CStaticModelEntity`, the second condition of `CStaticModelEntity`, the two conditions of `CParticleEffectEntity`).
5. **Six classes do nothing:** `CSpawnEffectEntity` and `CSpawnEffectCapsuleEntity` print `Class (%1) is obsolete and should be removed from level ASAP.` when created (the same function as the obsolete brains, puppets and items); the movie player, the static vibration, the simple effect and the sound effect exist in no world file (the last two are made by code: material hits, explosions, scheme sounds).
6. **A hard-coded level fix.** The path marker with entity id 3325 in the world whose name contains `1_2_ApproachingVillage` is moved at boot to a fixed horizontal position (section 1 of the markers page).

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were listed and the 932 exported classes grouped by name (the split of T01-026). The 55 classes are the survey's "world content" part; their exported functions total **854** (500 of the 18 entity classes, 193 of the 18 properties classes, 6 of `CParticleEffectSound`, 155 of the 18 debug helpers), as the survey. Names are quoted as exported.
- **Class registration and member tables.** The DLL's member tables (the member name, the editor label, the type, the flags) were read for **29 classes of the family** by scanning the data sections for runs of member records, and cross-checked against the exported names (every class that has a table has exports of the same stem). 28 of them are saved in the data and 3 are not (`SSimpleEffectEntity` 20 members and its properties 16, `SSoundEffectEntity` 3 and 10, `SMoviePlayerentity` 6 and 4; the DLL's own spelling of the last is quoted). Byte offsets are **not recorded** (`conventions.md`). The remaining classes add no member. The flag `0x4000` marks a replicated member (`SMF_NETUPDATE`, `replication.md`), `0x2000` an owned object; the editor labels (for example "Glue hulls to bones") are quoted where they explain a member.
- **Virtual function tables.** The table of each of the 18 entity classes was compared slot by slot with `SBaseEntity`'s (105 slots; `CBaseEntity` has 102), and of the 18 properties classes with `SBaseProperties`'s (30 slots). The linker shares identical small functions, so a slot may show another class's name (the empty hooks of unrelated classes are one function); a hook is named in this page only when the class itself exports a method of that name or when the slot's role is fixed by other classes (section 2).
- **Methods.** All 526 functions exported by the 55 classes (the named methods and the default constructors) were decompiled; **the bodies of about 130 were read closely** (everything of `CBaseEntity` and `SBaseEntity` that is not a one-line forwarder, the creation, boot, net and state functions of the entity classes, every function of the small classes) and the others were **profiled** (size, the functions they call, the messages they print). Each method sentence carries its basis: **R** read, **P** profiled (structure and callees; the sentence is an inference), **N** name only. Each rule is **medium** (one reading) unless the data confirms it, which is said.
- **Data.** `tools/re/worldcontent_classes_verify.py` (about 15 minutes: 2 for the type tables of all 8,063 format-8 files, 13 for the 59 worlds) prints every count and table of the five pages: which of the 55 classes are saved, the member order check, the census of the placed entities and records of the family with the value of each member, the copy checks between entity and record, the flag checks, the constructor-default checks and the relations (parents, links).
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run (no timing, no random sequence, no net traffic); the bodies of the longest creation and destruction functions (`OnCreate` of the static model and the prop, `DestroyOnePhaseLoop`, `PostReceiveUpdate`) were profiled only; the 20 environment sound presets, the sound flags, the effect-life types and the material effect tables are in other specs or open (Open questions).

## Layout / rules

### 1. The 55 classes and the data

"Saved as" is the class name in the type tables of the files; "records" are the property records of the extra section (`wld-entities.md`); counts are entities over the 59 worlds (script section 3).

| Exported class | Parent | Saved as | Members (data / DLL) | Files | Entities (worlds) | Records |
|---|---|---|---|---|---|---|
| `CBaseEntity` | `CEntity` (engine) | `CBaseEntity` | 1 / 1 | 59 `.wld` | (all 73,098) | |
| `SBaseEntity` | `CBaseEntity` | `SBaseEntity` | 1 / 1 | 59 | (all) | |
| `CBaseProperties` | `CEntityProperties` | `CBaseProperties` | 1 / 1 (two class variants, 0 members in 26 files) | 67 (59 `.wld`, 8 `.ctl`) | | (all) |
| `SBaseProperties` | `CBaseProperties` | `SBaseProperties` | 1 / 1 | 67 | | |
| `CStaticModelEntity` / `Properties` | `SBaseEntity` / `SBaseProperties` | same names | 51 / 51, 18 / 18 | 59 | 25,383 (59) | 25,391 (8 owned) |
| `CPropEntity` / `Properties` | same | `SPropEntity` / `SPropProperties` | 44 / 44, 25 / 25 | 32 | 1,726 (32) | 1,762 (36 owned) |
| `CParticleEffectEntity` / `Properties` | same | **`CEffectEntity` / `CEffectProperties`** | 18 / 18, 8 / 8 | 57 | 2,352 (57) | 2,347 |
| `CParticleEffectSound` | none (structure) | same | 7 / 7 | 58 (57 `.wld`, 1 `.rsc`) | | |
| `CShakerEffectEntity` / `Properties` | same | `SShakerEffectEntity` / `Properties` | 3 / 3, 2 / 2 | 7 | 12 (7) | 12 |
| `CStaticSoundEntity` / `Properties` | same | `SStaticSoundEntity` / `Properties` | 19 / 19, 13 / 13 | 50 | 610 (50) | 610 |
| `CEnvSFXEntity` / `Properties` | same | same names | 4 / 4, 5 / 5 | 44 | 136 (44) | 136 |
| `CFlareEntity` / `Properties` | same | `SFlareEntity` / `Properties` | 4 / 4, 7 / 7 | 48 | 2,501 (48) | 2,501 |
| `CPathMarkerEntity` / `Properties` | same | `SPathMarkerEntity` / `Properties` | 5 / 5, 2 / 2 | 47 | 10,639 (47) | 10,639 |
| `CVisBoxEntity` / `Properties` | same | `SVisBoxEntity` / `Properties` | 1 / 1, 3 / 3 | 44 | 362 (44) | 362 |
| `CNavMeshMarkerEntity` / `Properties` | same | same names | 1 / 1, 0 / 0 | 15 | 54 (15) | 54 |
| `CSimpleEffectEntity` / `Properties`, `CSoundEffectEntity` / `Properties`, `CMoviePlayerEntity` / `Properties` | same | **not saved** (the DLL declares `SSimpleEffectEntity` 20 members, `SSimpleEffectProperties` 16, `SSoundEffectEntity` 3, `SSoundEffectProperties` 10, `SMoviePlayerentity` 6, `SMoviePlayerProperties` 4) | | | 0 | 0 |
| `CStaticVibrationEntity` / `Properties`, `CSpawnEffectEntity` / `Properties`, `CSpawnEffectCapsuleEntity` / `Properties` | same | **not saved**, no member table at all | | | 0 | 0 |
| the 18 `CDebug...` classes | none | **not saved** | | | | |

Data classes of the same part of the hierarchy that **no export of this family covers** (script section 1): `CTextEffectEntity` (base `CBaseEntity`, 5 members) and `CTextEffectProperties` (10 members; **14** placed entities and records in 11 worlds; the DLL declares their member tables but exports no function for them, the text effect is the engine's `CTextEffect` machinery, `effects-pipeline.md`), and the engine bases `CEntity` (5 members) and `CEntityProperties` (9 members; `wld-root.md`). A reader must not look for the 30 not-saved classes of the table in a world file.

### 2. `CBaseEntity`, `SBaseEntity` and the two properties bases

**Role.** `CBaseEntity` is the game's entity base: it sits on the engine's `CEntity` (the engine class that owns the serial `CEntity.1`, the class pointer, the think and step scheduling and the world links, `wld-root.md`) and adds the game-wide hooks (damage, tools, objects the player can use, attachments, view points). `SBaseEntity` adds the chapter link and the begin and end hooks of the S layer; **most placed entities of the data derive from it**: of the 94 entity classes of the worlds, 12 sit directly on `CBaseEntity` (bloom, cube background, diamond hand, directional light, fluid, force field, formation marker, haze, macro, point light, spawn marker, text effect) and the camera `SCameraEntity` is a second direct child of `CBaseEntity` beside `SBaseEntity` (`classes-catalog.md`; the catalogue gives 13 direct children besides `SBaseEntity`). The properties bases do the same for records.

**Members** (script section 2: counts equal, the one named data member at its binary position):

| Class | Binary name | Data name | Type | Meaning |
|---|---|---|---|---|
| `CBaseEntity` | `en_ulBaseFlags` | `CBaseEntity.1` | `BaseEntityFlags` | bit flags of the entity: `BEF_SHUTEDDOWN` (1, set while the entity is shut down) and `BEF_IGNOREDBYNAVIGATION` (2). **Data:** 0 in every placed entity except 2 in exactly the static models whose record has "Ignored by navigation" (script section 3). **high** for value 2 (data), medium for 1 (name only) |
| `SBaseEntity` | `en_henChapter` | `SBaseEntity.1` | handle to a chapter info entity | the chapter the entity belongs to; `OnCreate` and `OnUpdate` copy it from the record's `ep_hepChapter` (the chapter info record's entity); **null in 54,627 of 57,529 placed entities, set in 2,902** (`wld-entity-schema.md`; script section 3). `GetChapter` returns it. **high** |
| `CBaseProperties` | `ep_ulEntityIDVis` | `CBaseProperties.ep_ulEntityIDVis` | `ULONG` | the entity's serial for visibility; the constructor sets all ones and `mdPostRead` copies `CEntityProperties.8` into it (so it equals the serial wherever the record has it: 51,703 of 51,704 records, `wld-visibility.md`). Absent in 17,473 records of older worlds (the second class variant) |
| `SBaseProperties` | `ep_hepChapter` | `SBaseProperties.1` | handle to a chapter info record | the chapter of the record; 3,279 set, `wld-entities.md` |

**Hooks.** `CBaseEntity` has **102 virtual slots**, `SBaseEntity` and its children **105** (three new: `ContributeToGameTotalStats` and the S layer's `OnBegin` and `OnEnd`, all empty in the base). The slots the family uses, with the class that fixes the role (**medium**, from the names of the overrides):

| Slots | Hook | Base behaviour (`CBaseEntity`) | Overridden by |
|---|---|---|---|
| 5, 6, 7, 8 | `OnCreate`, `OnNetCreate`, `OnUpdate`, `OnDelete` | forward to the engine's `CEntity`; `OnNetCreate` calls `OnCreate` (R) | all 16 entity classes override `OnCreate`; 12 override `OnUpdate` and `OnDelete` |
| 11, 12 | `OnHullTouched`, `OnFluidTouched` | forward to `CEntity` (R) | prop |
| 16, 18, 19, 55, 56 | `OnSimStart`, `OnSpawn`, `OnStep`, `OnShutDown`, `OnBoot` | forward or empty (R); `SBaseEntity::OnSpawn` calls the virtual after it (R) | `OnBoot`: 10 classes; `OnShutDown`: 7; `OnStep`: model, prop, static sound |
| 20, 21, 39 | `SetAbsPlacement`, `GetAbsPlacement`, `GetBoundingBox` | `CEntity`'s; the placement of a model entity is its renderable's | effects, markers, model, prop |
| 24 | `CollectForSortedIteration` | forwards (R) | static model, particle effect, flare |
| 27, 29 | `GetNetBehavior`, `GetRelevantDistance` | `NEB_NONE`; a very large constant (R) | section 4 |
| 28 | `GetUpdatePriority` | a constant (R) | prop (distance based) |
| 36, 37, 38 | `PreSendUpdate`, `AdjustBaseline`, `PostReceiveUpdate` | empty | model, prop, particle effect, static sound |
| 40 | `RenderRanges` | forwards to `CEntity` (R) | shaker, static sound, vibration, path marker (editor drawing of ranges) |
| 41-50 | `GetBoundingBoxForAutoAiming`, `GetHealthColor` (constant `0xff7f7fff`), `GetViewOrigin`, `GetViewTarget`, `GetAspectForAttaching`, `GetAttacmentAbsolutePlacement`, `GetAttacmentRelativePlacement`, `GetThreatSoundOrigin` | placement-derived defaults (R) | path marker (`GetViewOrigin`), model, prop (`GetAspectForAttaching`) |
| 53, 61, 62, 64, 90, 91, 95, 96, 101, 102 | `InflictDamage_EDT`, `CanReceiveRangeDamage`, `OnReceiveDamage`, `OnDie`, `IsDestructible`, `ExecuteDestruction`, `GetPiercingProjectileResilience`, `BeDestroyedByPeircingProjectile`, `IsDestroyed`, `ContributeToGameTotalStats` | `GetPiercingProjectileResilience` returns −1 (R); the others are empty or a shared constant stub (R) | model, prop (all), flare and particle effect (`ExecuteDestruction`) |
| 58, 66-76, 92, 93 | `Teleport` (returns 0), `GetObjectManipulationFlags`, `GetObjectManipulationMessage`, `EnableUsage`, `DisableUsage`, `GetTool`, `CanBePickedAsTool`, `OnPickedAsTool`, `OnThrownAsTool`, `GetToolModelInstance`, `GetToolAbsPlacement`, `GetToolRelPlacement`, `OnBeenTaken`, `OnBeenThrown` | see below | model (use), prop (tool) |
| 78, 81, 85, 86 | `SetSpawner`, `GetMechanism`, `SetVelocity`, `SetAngVelocity` | empty or constant stubs | prop, model |
| 82-89 | `GetMass` (the mechanism's mass, or a constant), `GetVelocity` (the mechanism's, else zero), `GetAngVelocity` (zero), `DoJump`, `ReceiveForceImpulse` (adds an impulse to the given body) | R | |
| 63 | `GetChapter` | (`SBaseEntity`) the chapter handle (R) | |

**Named methods of `CBaseEntity` (111)**, grouped (basis **R/P/N**):

| Group | Methods | What they do |
|---|---|---|
| life cycle | `OnCreate`, `OnNetCreate`, `OnUpdate`, `OnDelete`, `OnBoot`, `OnSimStart`, `OnSpawn`, `OnStep`, `OnShutDown`, `OnDie`, `SetNextThink`, `ClearNextThink`, `EnableOnStep`, `DisableOnStep`, `PreWorldChange`, `PostWorldChange`, `UpdateTemps` | R: forward to the engine's `CEntity` or empty; the constructor sets the flags to 0. `OnNetCreate` calls the virtual `OnCreate` |
| delete and destroy | `Delete`, `DeleteChildren`, `Destroy`, `DestroyChildren` | `Destroy(amount)` raises a macro event on the entity (the event id was not extracted) and then calls `DestroyChildren` (P); `DeleteChildren` and `DestroyChildren` apply the call to the entities found by `GetChildEntities` (P) |
| hierarchy | `AddFirstLayerChildEntities`, `GetChildEntities`, `macGetParent` | the entities whose parent aspect belongs to this entity; `macGetParent` is the macro function (P) |
| placement and motion | `SetAbsPlacement`, `GetAbsPlacement`, `GetBoundingBox`, `GetViewOrigin`, `GetViewTarget`, `GetAttacmentAbsolutePlacement`, `GetAttacmentRelativePlacement`, `GetThreatSoundOrigin`, `GetMass`, `GetVelocity`, `GetAngVelocity`, `SetVelocity`, `SetAngVelocity`, `Teleport`, `DoJump`, `ReceiveForceImpulse` | R; the two attachment functions ask the model renderable (`GetModelRenderable`, slot 47) for the placement of an attachment, false when there is no model |
| macros | `macGetPlacement`, `macSetPlacement`, `macGetLinearVelocity`, `macGetAngularVelocity`, `macSetLinearVelocity`, `macSetAngularVelocity`, `macGetAttachmentAbsolutePlacement`, `macGetAttachmentRelativePlacement`, `macGetParent`, `macInflictDamage` | the functions the macro language can call on any entity (`wld-macro-vocab.md`); R: the setters forward to the virtual functions, and **`macSetAngularVelocity` takes degrees and converts to radians** (the same constant as in the placement code) |
| class queries | `IsOfClass`, `IsDerivedFromClass` | R: `IsOfClass(name)` is true only for the entity's own exact class, `IsDerivedFromClass(name)` for the class or a parent (both by class name) |
| searches | `GetClosestEntity`, `GetClosestLivingPuppet`, `GetClosestDeadPuppet` | R: scan all entities of the world with a small iterator object (origin, class or puppet-alive flag, maximum range; the default range is about 3.0e38) and return the nearest; `GetClosestEntity` returns an invalid handle for an invalid class id |
| tools | `GetTool`, `CanBePickedAsTool`, `OnPickedAsTool`, `OnThrownAsTool`, `GetToolAbsPlacement`, `GetToolRelPlacement`, `GetToolModelInstance`, `OnBeenTaken`, `OnBeenThrown`, `GetObjectManipulationFlags`, `GetObjectManipulationMessage`, `EnableUsage`, `DisableUsage` | R: **`GetTool` returns the character-tool object that the global game parameters hold**; `CanBePickedAsTool` is true when the entity has a model renderable and that renderable is not hidden; `GetObjectManipulationMessage` returns the text `Press {plcmdUse} to use` when the manipulation flags have `OMF_SHOW_MESSAGE` (8) and `OMF_CAN_BE_USED` (1), `... to take` for `OMF_CAN_BE_TAKEN` (2), else an empty text (the other flag, `OMF_DISABLE_CENTERING`, is 4); the placement of a carried tool is the entity's placement as a matrix |
| damage | `OnReceiveDamage`, `InflictDamage_EDT`, `macInflictDamage`, `CanReceiveRangeDamage`, `IsDestructible`, `IsDestroyed`, `ExecuteDestruction`, `BeDestroyedByPeircingProjectile`, `GetPiercingProjectileResilience`, `ForcesPiercingProjectileToExplode`, `CanBeKicked`, `IsAimable`, `IsAutoAimable`, `GetHealthColor`, `OnSeeTarget`, `OnNotSeeTarget`, `OnTrigger` | defaults only (R/N); the two inflict functions are the script-callable "do damage" (P) |
| net | `GetNetBehavior`, `GetRelevantDistance`, `PreSendUpdate`, `PostReceiveUpdate`, `AdjustBaseline` | defaults only; section 4 |
| other | `GetUpdatePriority`, `GetChapter` (S layer), `GetModelRenderable`, `GetModelInstance`, `GetMechanism`, `GetAllAspects`, `GetAspectForAttaching`, `CanSpawn`, `SetSpawner`, `PrepareForLaunch`, `YouWillBeMySpawnSource`, `AddDebugEntites`, `Render2D`, `RenderOpaque`, `RenderTranslucent`, `RenderRanges`, `CollectForSortedIteration`, `mdPostCopy`, `mdPostRead`, `mdScriptPostRead` | defaults (shared stubs whose value is not distinguished: **39 of the 111 share one empty or constant function**); `mdPostCopy` and `mdPostRead` call the virtual `SetAbsPlacement`-related post-read hook (P) |

**`SBaseEntity`** (11 named): `OnCreate` and `OnUpdate` copy the chapter handle from the record (R, section above); `OnSpawn` calls its virtual hook (R); `GetChapter` returns the handle (R); `ContributeToGameTotalStats`, `OnBegin`, `OnEnd`, `OnBoot`, `OnShutDown`, `OnSimStart`, `OnDelete` are empty (R). **`CBaseProperties`** (19 named): `GetBoundingBox` is the fixed box −0.5..0.5 by 0..1 by −0.5..0.5 (R), `GetModels` and `GetEnvelopeProviders` ask the model renderable (R), `GetShellOpenPaths` asks the model configuration (R), `GetPropertiesToSpawn` returns the record itself (R), `GetParentAspect` is the parent record's entity's aspect for a part (R), `GetEffectiveEntity` forwards to the engine (R), `OnStretch` returns false, `GetSpawneeCountLimitType` a constant, `IsMultiSpawnerTemplate`, `BreakSpawnerLinks`, `BreakMultiSpawnerLinks`, `MarkAsSpawnee`, `IsImportant` shared stubs (R). `SBaseProperties` has only the constructor, which sets the chapter handle to null.

### 3. The member-order rule and the record-to-entity pattern

**Rule (high for 28 classes, script section 2).** For every data class whose member table the DLL also declares, the data's members in **file order** and the binary's members in **declaration order** have the same count and agree position by position: every member the data names (`en_...`, `ep_...`) stands at the position of the same name in the binary's table, every data type fits the Hungarian prefix of the binary's name (`b` BOOL, `f` FLOAT, `id` IDENT, `qv` QuatVect, `v` Vector3f, `tm` Time, `sl` SLONG, `col` COLOR) and handle members are handles. The numbered members are therefore named by position; the five pages give each class's table. The record has its own table in the same way.

**Pattern.** For the classes with data the creation function reads the record's members, so that the **entity's `en_` member equals the record's `ep_` member** in every pair (script section 3 prints each pair's tally: static model 8 pairs, prop 15, particle effect 4, static sound 8, flare 1, environment sound 1, shaker 1). The records are the editor's description, the entities hold a runtime copy; a reader that needs only the design data can read the records (`wld-entities.md`).

### 4. Net behaviour of the family (resolves three items of `replication.md`)

| Class | `GetNetBehavior` | Replicated members (`0x4000`) | Basis |
|---|---|---|---|
| `CBaseEntity`, `SBaseEntity` | `NEB_NONE` (0, inherited from the engine) | 0 of 1, 0 of 1 | R |
| `CStaticModelEntity` | `NEB_NONE` when the entity has no model renderable; else **`NEB_STATIC` (1) when `en_bControlByMacro` is set, or when the model configuration has a destruction holder with at least one configuration**; else `NEB_NONE` | **15 of 51**: `en_pmcCurrentModel`, `en_bHidden`, `en_mfFlags`, `en_bAnimPlayedByMacro` and the 11 `en_...Net...` animation members (action, name, fade-in, cut-back, fade-in on a new state, flags, speed multiplier, strength, wait, loop count, current offset) | R |
| `CPropEntity` | **`NEB_DYNAMIC` (2) when there is no spawner and the entity is not a multi-spawner template, else `NEB_STATIC`** | **20 of 44** (`replication.md` lists them) | R |
| `CParticleEffectEntity` | `NEB_STATIC` when the parent entity (`en_henParent`) reports itself destructible; else `NEB_STATIC` when `en_bAutoStart` is **false**, `NEB_NONE` when it is true | 2 of 18: `en_pesNetEffectState`, `en_bNetEmitting` | R |
| `CSimpleEffectEntity` | the instance's own member `en_nebNetBehavior` (default 0 = none; the creator chooses) | **16 of 20** (the whole effect) | R |
| `CStaticSoundEntity` | `NEB_STATIC` when `en_bAutoPlay` is **false**, else `NEB_NONE` | 3 of 19: `en_sssNetState`, `en_fFadeInLength`, `en_fFadeOutLength` | R |
| `CSoundEffectEntity`, `CShakerEffectEntity`, `CEnvSFXEntity`, `CMoviePlayerEntity`, `CStaticVibrationEntity`, `CFlareEntity`, `CPathMarkerEntity`, `CVisBoxEntity`, `CNavMeshMarkerEntity`, `CSpawnEffectEntity`, `CSpawnEffectCapsuleEntity` | `NEB_NONE`: none of them overrides it | 0 | R (no override in the table) |

So the geometry, the markers and the sounds that start by themselves are **never sent**: every machine builds them from its own data, and only script-driven state is replicated (a macro-controlled model, a destructible model, a prop that moves, a sound started by a macro). The `NEB_` values are the engine's (`replication.md`: 0 none, 1 static, 2 dynamic, 3 predicted).

### 5. The 55 classes by what they are for

Section 1 of each companion page. The editor-side strings are the DLL's own: the placement dialog names (for example "Particle effect", "Shake effect", "Environment sound effect", "Shaker entity") and thumbnails (`ParticleEffect`, `ShakeEffect`, `EnvSFX`, `ExternalJoint`) are quoted where they identify a class.

## Invariants

1. 55 exported classes = 18 entity + 18 properties + 1 value structure + 18 debug helpers; 854 exported functions; **25 saved** (23 by name or `S` twin, 2 by the registered name `CEffectEntity`, `CEffectProperties`), 30 not (script section 1). A reader must not look for the 30 in a world file.
2. For every saved class the data's member count equals the binary's (28 of 28), the named members stand at the binary position and the types fit (script section 2).
3. Each placed entity has a record of the matching stem and the pairs of section 3 agree in **every** entity (script section 3 prints the tally; every pair is `equal` except the parent part of 2 static models whose entity has no parent).
4. The class defaults of section 3 hold for every placed entity (script section 3).
5. `CBaseEntity.1` is 2 exactly for the static models that are ignored by navigation (script), else 0.
6. No class of the family overrides `GetNetBehavior` except the six of section 4; a reader must treat every other class of the family as unreplicated.

## Open questions

Ranked by what blocks an implementer. IDs are **new** (appended to `docs/roadmap/T01-reverse-engineering.md`).

1. **The animation, destruction and macro behaviour of static models and props in full** (`OnCreate`, `PostReceiveUpdate`, `SetupCollision`, `DestroyOnePhaseLoop`, `ExecuteDestruction`, the death spawner, the anim-event handler, the model destruction data): **T01-175** (statics), **T01-176** (props: physics mechanism, collision and splash effects, tool handling).
2. **Effects**: the effect-life types, the sound flags, the three effect creation functions and the material effect tables, the text effect entity: **T01-177**, **T01-178**.
3. **Sound, vibration, environment and movies**: the static sound state machine in detail, the 20 `SfxEnvironment` values and what the sound environment does, who creates the movie player and the static vibration: **T01-179**, **T01-180**, **T01-181**.
4. **Markers and debug**: the consumers of the path marker, the nav-mesh marker's call on the world info entity, the hard-coded marker move, the debug function family `samDbg*` and the dump configuration: **T01-182**, **T01-183**.
5. **Run-time checks on the original** (net behaviour of the family, destruction cascade, effect lifetimes, sound fades; numbers only): **T01-184**.
6. **The remaining `CBaseEntity` macro functions and the shared default stubs** (the value each of the 39 shared empty or constant defaults returns; the full macro list as the macro language names it): **T01-185**.

## Test vectors

All from the retail install via `tools/re/worldcontent_classes_verify.py`, unless a binary reading is named.

1. **Counts.** 55 exported classes, 854 exported functions, 471 named methods; 25 saved (11 under the `C` name, 12 as the `S` twin, 2 under the registered name), 30 not saved; 43,775 placed entities of the 55 classes (43,789 with the 14 text effects) in the 59 worlds, with 43,828 property records.
2. **Registered name.** `CParticleEffectEntity` is saved as `CEffectEntity`: 18 members both in the file's type table and in the DLL's table; the constructor's defaults (`en_bAutoStart` 1, `en_bEmitting` 1, `en_bNetEmitting` 1, the state 0) equal the data of every placed effect that has not started yet.
3. **Order rule.** `CStaticModelEntity` data members in file order `.1, .2, .10, .11, .12, .3, .4, .5, .6, .20, .21, .22, en_hsmaedAnimEventsData, ...` are `en_hmrModel, en_hhuHull, en_hmech, en_bCollision, en_idMechanism, en_mlidMacroLatentID, en_fSleepTime, en_idAnimToStop, en_pmcOrgModel, en_idAutoStartAnim, en_henParent, en_idParentPart, en_hsmaedAnimEventsData, ...`.
4. **Flags.** A static model whose record has `ep_bIgnoredByNavigation` 1 has `CBaseEntity.1` = `BaseEntityFlags=2`; one whose record has `ep_bDisableDestruction` 1 has `en_mfFlags` = 65536.
5. **Net.** A static model that has `en_bControlByMacro` 1 is `NEB_STATIC`; a static sound with `en_bAutoPlay` 1 is `NEB_NONE`; a prop that a spawner made (`en_henSpawner` set) is `NEB_STATIC`, a free prop `NEB_DYNAMIC`.
