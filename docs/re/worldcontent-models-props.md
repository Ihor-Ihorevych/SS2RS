# Static models and props of `Sam2Game.dll` (T01-110, part of T01-026)

- **Roadmap task:** T01-110 (overview [`worldcontent-classes.md`](worldcontent-classes.md); this is the page for `CStaticModelEntity`, `CStaticModelProperties`, `CPropEntity` and `CPropProperties`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the member tables, the member-order rule, the counts, the copies from record to entity, the flag values and the construction defaults (all checked on every placed entity with `tools/re/worldcontent_classes_verify.py`); **medium** for the hooks, the net rules, the damage flags and the macro functions (each is one reading of a short function; **nothing was run**); **low** for every method marked `N` or `P` and for the open questions.

Prerequisites: [`worldcontent-classes.md`](worldcontent-classes.md) (marks `R`, `P`, `N`, the member-order rule, the base class), `wld-geometry.md` (static models as level geometry, the model renderable), `destruction.md` and `destruction-models.md` (the destruction data of a model), `physics.md` (mechanisms), `replication.md`, `wld-macro-vocab.md` (the macro functions).

## Summary

- **`CStaticModelEntity`** is the placed model: **25,383 entities in all 59 worlds** (35 % of the 73,098 entities), each with a record of the same stem. It is the level's scenery, its animated and its destructible parts. It owns a model renderable and, when "Collision" is on (21,420 of 25,383), a physics mechanism; it can play animations, disappear and appear, be used by the player and be destroyed. Saved under its exported name.
- **`CPropEntity`** is the free physical object (crate, barrel, ...): **1,726 entities in 32 worlds**, always with a mechanism, a splash and a dust effect, and a destruction sound source. Saved as `SPropEntity`. A prop can be **made by a spawner** (36 are) and can be **carried as a tool** by the player.
- **The two classes share one family of members** (model, stretch, shader modifiers, parent part, cull distance, fade length, shadow type, death spawner, mechanism name, "Control by macro" or "Physics enabled") but not code: a static model is steered by macros and the record flags; a prop is a rigid body with a state that replicates (20 of 44 members).
- **Damage and destruction are flags plus the model's destruction data**: a model is destructible unless its record says "Disable destruction" (2,064 static models); damage can be ignored, reported or limited to kicks by macro functions (section 2.4).
- **Net.** A static model is replicated only when it is macro-controlled (536) or has a destruction holder; a prop is dynamic unless a spawner made it (overview, section 4).

## Evidence

- **Export table.** `CStaticModelEntity` 81 symbols (74 named), `CStaticModelProperties` 14 (7), `CPropEntity` 61 (54), `CPropProperties` 19 (12): **175** exported functions, 147 named methods.
- **Member tables.** `CStaticModelEntity` (51 members), `CStaticModelProperties` (18), `SPropEntity` (44), `SPropProperties` (25) were read from the DLL (names, labels, flags) and matched to the data by the order rule: all four agree on count, named members and types (script section 2). Byte offsets are not recorded.
- **Methods.** Read in full: the constructors, `GetNetBehavior`, `GetRelevantDistance`, `OnBoot`, `OnStep`, `EnableAnimationEvaluation`, `CheckCapabilityForExecutingMacro`, `OnReceiveDamage`, `Destroy`, `IsDestroyed`, `IsDestructible`, `DoAppear`, `DoDisappear`, `GetObjectManipulationFlags`, `CanReceiveRangeDamage`, `GetPiercingProjectileResilience` and the prop's `GetNetBehavior`, `OnHullTouched`, `OnFluidTouched`, `OnReceiveDamage`, `ExecuteDestruction`, `AdjustBaseline`, `OnBeenTaken`, `OnShutDown`, `OnPickedAsTool`, `OnThrownAsTool`, `SetSpawner`, `GetUpdatePriority`. `OnCreate` (1,544 and 3,204 instructions), `PostReceiveUpdate`, `PreSendUpdate`, `ApplyClientOffsetCorrection`, `SetupCollision`, `ResetDynamicMembers`, `DestroyOnePhaseLoop`, `ChangeToModel` and the macro functions `PlayAnim...` were **profiled** (callees, strings) and their rules read only at the level of "copies the record and builds the renderable".
- **Data.** Script sections 2 and 3: every count, the member tables, the 15 + 8 entity-record pair checks, the construction defaults, the flag cross tables.
- **Not checked:** nothing was run; the destruction data (`CModelDestructionCustomData`, `destruction.md`), the animation queue and the material effect tables were not read.

## Layout / rules

### 1. `CStaticModelEntity`

**Role.** One model in the world. `OnCreate` copies the record, loads the model (or builds an empty model with one mesh when the record has none), makes a **model renderable** (stretch, fade length, cull distance, shader modifiers, shadow type), sets the placement, builds the **physics mechanism** when "Collision" is set, and makes two **sound sources** (`en_hssAlive`, `en_hssDestruction`). Two readings agree that the mechanism exists exactly when the record's "Collision" flag is 1: **21,420 of 21,420** collision records have a mechanism handle, **3,963 of 3,963** others have none (script). The hull handle `en_hhuHull` is null in all 25,383 (the hull is part of the mechanism).

**Members** (51; `#` the data's number, `@` owned object, **R** replicated). The defaults are the constructor's (high where "all 25,383" is given: every placed entity stores the default).

| # | Binary name | Meaning | Data |
|---|---|---|---|
| 1 `@` | `en_hmrModel` | the model renderable | set in all |
| 2 `@` | `en_hhuHull` | the model hull | null in all |
| 10 `@` | `en_hmech` | the physics mechanism | set 21,420, null 3,963 |
| 11 | `en_bCollision` | "Collision": a mechanism exists | equals the record in all 25,383 |
| 12 | `en_idMechanism` | the mechanism's name (identifier) | equals the record in all |
| 3 | `en_mlidMacroLatentID` | the macro wait that a `PlayAnimWait` or `LatentSleep` is blocked on | structure |
| 4 | `en_fSleepTime` | time the entity sleeps (`LatentSleep`) | 0 in all |
| 5 | `en_idAnimToStop` | the animation that `EndLatentAnim` removes | empty |
| 6 | `en_pmcOrgModel` | the model as created (`ChangeToModel` can replace it; `IsDestroyed` compares) | set |
| 20 | `en_idAutoStartAnim` | animation started at boot | equals the record in all |
| 21 | `en_henParent` | parent entity (attachment) | set 4,778: models 4,321, animated movers 452, doors 4, a character 1 |
| 22 | `en_idParentPart` | the parent's part (bone) | equals the record in 25,381, differs in 2 (the entity has no parent) |
| named | `en_hsmaedAnimEventsData` `@` | the holder of the animation-event sound and effect lists (created on demand by `EnableAnimationEvaluation`) | set 43 |
| named | `en_fSoundHotSpot`, `en_fSoundFallOff` | hot spot and fall-off of the model's sounds | equal the record in all (hot spot 0..100, fall-off 0..200) |
| named | `en_bMoveHulls` | "Glue hulls to bones" | 110 set; equal the record |
| named | `en_slAccuDamage`, `en_tmLastDamage`, `en_henLastDamager`, `en_slLastDamage` | the damage received so far, when, from whom, and the last amount | 0 / null in all |
| named | `en_henDeathSpawner` | the spawner that fires when the model is destroyed (`OnBoot` marks it a death spawner) | set 219 |
| named | `en_hssAlive`, `en_hssDestruction` `@` | sound sources of the living model and of the destruction | set in all |
| named | `en_qvOriginalPlacement` | where it was placed | structure |
| named | `en_smaaAnimAction`, `en_idAnimName`, `en_fAnimFadeInTime`, `en_fAnimCutBackTime`, `en_ulAnimFlags`, `en_fAnimSpeedMultiplier`, `en_fAnimStrength`, `en_bAnimWait`, `en_fAnimLoopsCount`, `en_fAnimFadeInNewState` | the **last animation command** a macro gave (the action is `StaticModelAnimAction`: 0 none, 1 play, 2 stop, 3 stop all, 4 new clear state, 5 new cloned state, 6 remove) | constructor defaults in all 25,383: action 0, fade 0.2, cut-back 0, flags 0, speed 1, strength 1, wait 0, loops −1, new-state fade 0.2 |
| named | `en_fClientOffsetError` | the client's animation-time error that `ApplyClientOffsetCorrection` removes | 0 |
| named | `en_bControlByMacro` | "Control by macro" | 536 set; equals the record in all |
| 100 **R** | `en_pmcCurrentModel` | the model now shown | null in all (the net copy) |
| 101 **R** | `en_bHidden` | the model is hidden (after `Disappear`) | 0 in all |
| named **R** | `en_mfFlags` | `ModelFlags`, see below | 65,536 in 2,064 (= the records with "Disable destruction"), 0 in 23,319 |
| named **R** | `en_bAnimPlayedByMacro` | the last animation was started by a macro | 0 |
| named **R** | `en_smaaNetAnimAction` ... `en_fNetCurrentAnimOffset` | the 11 net copies of the animation command and the current animation offset (the replicated state) | constructor defaults in all |

**`ModelFlags`** (the DLL's names; **medium**): `MF_CAN_BE_USED` (1: the player can use the model, `EnableUsage` and `DisableUsage` set it), `MF_REPORT_DAMAGE` (2: raise the macro event once on the next damage), `MF_IGNORE_DAMAGE` (4: set while a destruction is running), `MF_IGNORE_KICKING_DAMAGE` (8: kicks do nothing), `MF_IGNORE_NON_KICKING_DAMAGE` (16: only kicks hurt), `MF_DISABLE_KICKED_DESTRUCTION_CASCADING` (32: a kicked model passes zero damage on), `MF_PROPERTIES_DISABLE_DESTRUCTION` (**65,536, high: the data**; copied from the record's "Disable destruction"). The values 1 to 32 follow from the DLL's list order and the bit tests in `OnReceiveDamage` and `GetObjectManipulationFlags` and are **medium**.

**Record** (`CStaticModelProperties`, 18 members; labels are the DLL's): `.1` `ep_pmcModel` "Model"; `.2` `ep_vStretch` "Stretch"; `.3` `ep_smlModifiers` "Shader modifiers"; `.4` `ep_bCollision` "Collision" (default 1); `.12` `ep_idMechanism` "Mechanism"; `.5` `ep_fCullDistance` "Cull distance" (default −1); `.6` `ep_fFadeLength` "Fade length" (−1); `.7` `ep_idParentPart` "Parent part"; `.8` `ep_idAutoStartAnim` "Auto start animation"; `.9` `ep_estShadows` "Shadows" (`EntityShadowType`: 0 none 24,167, 1 static 55, 2 dynamic 1,161); `ep_fSoundFallOff` "Sound fall off" (default 50) and `ep_fSoundHotSpot` "Sound hot spot" (10); `ep_bMoveHulls` "Glue hulls to bones"; `ep_hepDeathSpawner` "Death spawner" (record 220 set) and `ep_henDeathSpawner`; `ep_bDisableDestruction` "Disable destruction" (2,064); `.14` `ep_bIgnoredByNavigation` "Ignored by navigation" (492); `ep_bControlByMacro` "Control by macro" (536).

**What `OnCreate` does with the record (R for the copies, P for the order).** Mechanism name, collision flag, "Control by macro" and the sound ranges are copied to the entity; "Disable destruction" sets `MF_PROPERTIES_DISABLE_DESTRUCTION`; "Ignored by navigation" sets `BEF_IGNOREDBYNAVIGATION` in the base flags (**high: 492 of 492, and no other entity has the flag**); the shadow type sets a renderable flag (none = no shadow, static = static shadow, dynamic = none); the sound ranges are clamped to be non-negative with fall-off at least the hot spot (the clamp rewrites the record). The model renderable gets the entity as its owner.

**Net.** Overview section 4: `NEB_STATIC` when `en_bControlByMacro` is set or the model configuration has a destruction holder with at least one configuration, else `NEB_NONE`; `GetRelevantDistance` is the renderable's effective cull distance. `PreSendUpdate` fills the 11 net animation copies from the model's animation queue (the animation found, its start time) and `PostReceiveUpdate` applies them on the client (P: the strings and callees are the animation-queue ones; the exact transform is open, T01-175).

### 2. Behaviour of `CStaticModelEntity`

**2.1 Hooks it overrides** (vtable comparison with `SBaseEntity`; 117 slots, 12 new): `OnCreate`, `OnUpdate`, `OnDelete`, `OnStep`, `OnBoot`, `SetAbsPlacement`, `GetAbsPlacement`, `CollectForSortedIteration`, `GetNetBehavior`, `GetRelevantDistance`, `PreSendUpdate`, `PostReceiveUpdate`, `GetBoundingBox`, `GetAspectForAttaching`, `GetAllAspects`, `GetModelRenderable`, `CanReceiveRangeDamage`, `OnReceiveDamage`, `GetObjectManipulationFlags`, `EnableUsage`, `DisableUsage`, `GetMechanism`, `IsDestructible`, `ExecuteDestruction`, `GetPiercingProjectileResilience`, `BeDestroyedByPeircingProjectile`, `IsDestroyed`, `ContributeToGameTotalStats`. New hooks: `ApplyClientOffsetCorrection`, `SetupCollision`, `DeleteCollision`, `ResetDynamicMembers`, and the five animation-event hooks `OnAnimEventProduceSound`, `OnAnimEventProduceEffect`, `OnAnimEventProduceShakeEffect`, `OnAnimEventTerminateSound`, `OnAnimEventTerminateEffect` (the model's animation events play a sound, an effect or a shake and remember them so a later event terminates them; **P**).

**2.2 Life cycle.** `OnBoot` (R): marks the death spawner, applies the model to the renderable, and on the host (or when the entity does not hide itself) calls `DoAppear`; then plays the model's scheme sound for "alive" if the model has one. `OnStep` (R): while "Control by macro" is on and a mechanism exists, evaluates the mechanism's animation; also steps the animation-event holder and removes it when the model's animation queue has no events left; on a client it calls the net-apply hook; it disables stepping when nothing is left to do. `EnableAnimationEvaluation` (R) creates the event holder on demand and enables stepping. `OnUpdate` (P) re-reads the record (editor edit) and prints two diagnostics when a macro entity does not control a model flagged "Control by macro" (**"Static model entity with ID %1 is not controlled by any macro entity. Please turn off the ..."** and **"... can't be controlled by macro entity with ID %2 ..."**). `OnDelete` (P) frees the mechanism, the renderable tree and the two sound sources. `SetupCollision`, `DeleteCollision` (P) build or remove the mechanism (a mesh mechanism for the model's hulls, the category from the mechanism name, logs `StaticModelEntity::SetupCollision(): Model "..."` when a model lacks the hull it needs).

**2.3 Appear and disappear.** `Disappear` (macro) sets the renderable's hidden flag (`IsDisappeared` reads it) and `DoDisappear` also tells the virtual that removes collision (R); `Appear` clears the flag and re-creates the model at the original placement with the stored stretch (`DoAppear`, R). `OnBoot` appears the model on the host and, on a client, only when the entity is not hidden by its replicated flag.

**2.4 Damage and destruction.**

| Function | Rule (basis) |
|---|---|
| `CanReceiveRangeDamage` | true when the model has destruction data (R) |
| `IsDestructible` | true unless `MF_PROPERTIES_DISABLE_DESTRUCTION` is set (R) |
| `GetPiercingProjectileResilience` | the destruction data's value, unless destruction is disabled or absent, then the base's −1 (R) |
| `OnReceiveDamage(damage)` | if `MF_IGNORE_DAMAGE` is set do nothing; kicks (damage type 7) are ignored under `MF_IGNORE_KICKING_DAMAGE`, non-kicks under `MF_IGNORE_NON_KICKING_DAMAGE`; stores the last damage and damager; if `MF_REPORT_DAMAGE` is set, clears it and raises a macro event on the entity; asks the model's destruction rules how much damage is left (`ProcessModelReceiveDamage`); a kick under `MF_DISABLE_KICKED_DESTRUCTION_CASCADING` passes 0; a non-zero result calls `Destroy` (R) |
| `Destroy(amount)` | on the host it makes a script call so every machine runs it; on each machine it wakes the mechanism's physics, sets `MF_IGNORE_DAMAGE` while the base `Destroy` runs, then clears it (R) |
| `ExecuteDestruction`, `DestroyOnePhaseLoop`, `IsDestroyed` | the destruction chain: raise the death spawner, play the destruction sound, switch to the next destruction configuration of the model; `IsDestroyed` is true when the destruction configuration differs from the original model (R for `IsDestroyed`; P for the others; the log `Model %1 features infinite destruction loop.` is printed when a model's chain returns to itself) |
| `ContributeToGameTotalStats` | adds the death spawner's counts to the chapter totals (P) |

**2.5 Macro functions** (all call `CheckCapabilityForExecutingMacro` first: **a macro that changes the model's state while "Control by macro" is off prints a warning "Static model entity with ID %1 shouldn't execute macro function "%2". Turn on the flag "Control by macro" or this action won't be synced on client."**, R): `PlayAnim`, `PlayAnimStay`, `PlayAnimLoop`, `PlayAnimNLoops`, `PlayAnimWait` (blocks the macro until the animation ends), `StopAnim`, `StopAllAnims`, `AddAnim`, `RemoveAnim`, `NewClearState`, `NewClonedState`, `RandomizeAnimations` (not allowed on a server: "Macro function "RandomizeAnimations" doesn't work on server."), `Appear`, `Disappear`, `EnableUsage`, `DisableUsage`, `ReportDamage`, `LatentSleep`, `macDestroyOnePhase`, `macDisableKickingDamage`, `macDisableNonKickingDamage`, `macDisableKickedDestructionCascading`, plus the getters `GetLastDamager`, `GetLastDamage`, `IsDestroyed`, `IsDisappeared`, `ChangeToModel` (replaces the renderable's model; logs `CStaticModelEntity::ChangeToModel() Can't change to given model since renderable doesn't exist.`). Each stores its parameters in the last-command members (section 1) so the net copy can replay it (P).

**2.6 Use by the player.** `GetObjectManipulationFlags` returns 9 (`OMF_SHOW_MESSAGE` and `OMF_CAN_BE_USED`) when `MF_CAN_BE_USED` is set, else 0 (R); the message text is the base's "Press {plcmdUse} to use".

### 3. `CPropEntity`

**Role.** A model with a rigid body that the player pushes, shoots or carries. `OnCreate` (P, 3,204 instructions) copies the record, builds the model renderable and the mechanism, sets up the effects, creates the destruction sound source, and for a record that is a **spawner's template** or a "multi spawner template" it either spawns the spawner or prints `Spawner named: %1 is marked as multi spawner template and that cannot be.`

**Members** (44; **R** replicated; `@` owned). Binary names with the data number; all values from the script.

| # | Binary name | Meaning | Data |
|---|---|---|---|
| 2 `@` | `en_hre` | model renderable | set |
| 14 | `en_smlModifiers` | shader modifier list | structure |
| 5 `@` | `en_hmcMechanism` | the rigid body | set in all 1,726 |
| 6 | `en_henParentProp` | parent prop | null in all |
| 8 | `en_idParentPart` | parent part | equals the record |
| 9, 10 | `en_tmLastFluidTouch`, `en_tmLastGroundTouch` | when a fluid and the floor last touched it (rate limits for the effects: 0.25 s and 0.1 s) | structure |
| 24 | `en_idPoseAnimation` | "Appear pose animation" | equals the record |
| 50 `@` | `en_henSpawner` | the spawner whose template this is | set 36 |
| 51 | `en_henSpawnerWhoSpawnedMe` | the spawner that made this copy | null in all |
| 52 | `en_bMultiSpawnerTemplate` | multi-spawner template | 0 |
| named | `en_bNoGravity`, `en_bDynamicallyCreated`, `en_bPickedAsTool` | no gravity, made by game code, carried as a tool | 0 in all |
| named | `en_slAccuDamage`, `en_tmLastDamage`, `en_henLastDamager`, `en_slLastDamage` | damage record | 0 / null |
| named | `en_pfxSplash`, `en_pfxDust` | the splash effect (fluid contact) and the dust effect (collision) | set in all |
| named **R** | `en_pfFlags` | `PropFlags`: `PF_NONE` 0, `PF_FALLING`, `PF_SEEK_TARGET` (the DLL's names; the values are by order) | 0 in all |
| named | `en_ulDefaultObjectManipulationFlags`, `en_ulCurrentObjectManipulationFlags` **R** | what the player can do with the prop (`OMF_`: use 1, take 2, disable centering 4, show message 8) | 2 (take) in 1,668, 10 in 23, 3 in 21, others; the current equals the default; equal to the record in all |
| named | `en_henDeathSpawner` | spawner fired on destruction | set 839 |
| named `@` | `en_hssDestruction`, `en_hsaehAnimEventHandler` | destruction sound source (set), animation-event handler (null) | |
| named **R** | `en_idMechanismName`, `en_bSpawnEffects`, `en_fMinCollisionSpeedFX`, `en_fMaxCollisionSpeedFX`, `en_idLoopingAnimation`, `en_bActiveOnStart`, `en_fStretch`, `en_pmcDefaultModel`, `en_fFadeLength`, `en_fCullDistance`, `en_estShadows`, `en_bPhysicsEnabled`, `en_qvNetPlacement`, `en_vNetVelocity`, `en_bNetActive`, `en_pmcCurrentModel`, `en_bHidden`, `en_qvOriginalPlacement` | the record's settings and the net copy of the body (the 20 replicated members) | all equal the record (15 pairs checked in all 1,726); collision effects on in 1,558, speed range 3..8 m/s in all, active on start 18, stretch 0.116..7.5, physics enabled in all |

**Record** (`SPropProperties`, 25): "Model", "Stretch", "Shader modifiers", "Mechanism name", "Parent mechanism", "Parent part", "Cull distance", "Fade length", "Shadows", "Physics enabled", "Collision effects", "Min collision speed FX" (default 3), "Max collision speed FX" (8), "Looping life animation", "Appear pose animation", "Spawner" (36 set), "Multi spawner template", "Active on start", "No gravity", "Object manipulation" (default 2), and the death spawner (record 841 set). 1,762 records exist for 1,726 entities; the 36 extra are the templates behind spawners (`wld-entities.md`).

**Hooks it overrides** (108 slots, 3 new): `OnCreate`, `OnUpdate`, `OnDelete`, `OnHullTouched`, `OnFluidTouched`, `OnStep`, `SetAbsPlacement`, `GetAbsPlacement`, `GetNetBehavior`, `GetUpdatePriority`, `GetRelevantDistance`, `CopyToTemplate`, `CopyFromTemplate`, `PreSendUpdate`, `AdjustBaseline`, `PostReceiveUpdate`, `GetBoundingBox`, `GetAspectForAttaching`, `GetAllAspects`, `GetModelRenderable`, `OnShutDown`, `OnBoot`, `Teleport`, `OnReceiveDamage`, `GetObjectManipulationFlags`, `OnPickedAsTool`, `OnThrownAsTool`, `SetSpawner`, `SetVelocity`, `SetAngVelocity`, `ExecuteDestruction`, `OnBeenTaken`, `OnBeenThrown`, `GetPiercingProjectileResilience`, `BeDestroyedByPeircingProjectile`, `IsDestroyed`, `ContributeToGameTotalStats`; new: `ResetDynamicMembers`, `SetupCollision`, `DestroyOnePhaseLoop` (**medium**).

**Behaviour (R unless marked).**

| Function | Rule |
|---|---|
| `GetNetBehavior` | `NEB_DYNAMIC` when `en_bMultiSpawnerTemplate` is 0 and `en_henSpawner` is null, else `NEB_STATIC` |
| `GetUpdatePriority` | falls with the distance to the closest player (the distance is clamped to 1..200 m); **multiplied by a constant when the root body of the mechanism is asleep (its active flag is clear)** (the constants were not extracted, T01-176); a constant when there is no body |
| `OnHullTouched(...)` | when collision effects are on and the other hull belongs to a different mechanism: reads the relative speed from the touching body; at most every 0.1 s and only above `en_fMinCollisionSpeedFX`, spawns a material effect (`samSpawnEffectForMaterial`) scaled by `(speed - min) / (max - min)` clamped to 0..1 |
| `OnFluidTouched(...)` | when collision effects are on and at most every 0.25 s: spawns a `CSimpleEffectEntity` with the splash effect at the contact, its size from the relative speed |
| `OnReceiveDamage` | the same pattern as the static model: store the last damage, raise a macro event once when a flag is set, ask the model's destruction rules (`ProcessModelReceiveDamage`), `Destroy` on a non-zero result |
| `Destroy(amount)` | on the host, a script call to every machine; each machine wakes the physics in range and sets "ignore damage" around the base `Destroy` |
| `ExecuteDestruction` | not for a carried prop: fires the death spawner with the prop's placement, deletes the children, plays the destruction sound from the model's scheme if none is playing, runs one destruction phase and deletes the prop |
| `OnBeenTaken` | marks the prop "being manipulated" and raises a macro event; `OnPickedAsTool` hides the renderable, **deletes the mechanism** and sets `en_bPickedAsTool`; `OnThrownAsTool` unhides it and rebuilds the mechanism through `SetupCollision` (R) |
| `OnShutDown` | a spawned prop (spawner set, or dynamically created) is deleted; a free prop is put to sleep and hidden |
| `AdjustBaseline` | on a client nothing; on the host copies the placement and velocity members to a baseline entity when the prop is being manipulated (R) |
| `SetSpawner` | stores the spawner link if the argument is a spawner entity (R) |
| `EnablePhysics`, `DisablePhysics`, `WakeUp`, `GoToSleep`, `AttachToParentProp`, `ChangeToModel`, `macDestroyOnePhase`, `CopyToTemplate`, `CopyFromTemplate` | macro and spawner functions (P): physics on and off by creating or deleting the mechanism; wake and sleep activate and deactivate it; attach to a parent prop's body; `CopyToTemplate` and `CopyFromTemplate` copy only the shader modifier list to and from the template an editor spawner holds |
| `PreSendUpdate`, `PostReceiveUpdate` | fill and apply the 20 replicated members: placement, velocity, active flag, model and hidden flag (P) |

`SPropProperties` hooks (13 named): `Finalize`, `GetBoundingBox`, `GetModelRenderable`, `OnStretch`, `OnGetStretch` (the record's stretch), and the spawner link functions `BreakSpawnerLinks`, `BreakMultiSpawnerLinks`, `GetEffectiveEntity` (the spawner when the record is its template), `IsMultiSpawnerTemplate`, `MarkAsSpawnee`, `GetSpawneeCountLimitType`, `GetExecutionCommand_EDT` (editor command; P). `CStaticModelProperties` (7 named): `Finalize`, `GetBoundingBox`, `GetModelRenderable`, `OnStretch`, `OnGetStretch`, `IsModelHolder_EDT`, `GetExecutionCommand_EDT` (P).

## Invariants

1. Every placed static model has a record; **25,383 of 25,383** pairs agree on collision, mechanism name, auto-start animation, sound ranges, hull gluing and "Control by macro"; `en_idParentPart` equals the record in 25,381 and differs in 2 whose entity has no parent (script).
2. `en_hmech` is set exactly when `en_bCollision` is 1 (21,420 set, 3,963 not); `en_mfFlags` is 65,536 exactly when the record's "Disable destruction" is set (2,064); `CBaseEntity.1` is 2 exactly when "Ignored by navigation" is set (492).
3. Every placed prop has all four of: a mechanism, a splash effect, a dust effect and a destruction sound source; its current object-manipulation flags equal the default; 36 have a spawner.
4. The animation-command members of a placed static model are the construction defaults (script).
5. No static model has a hull handle, a damager or an animation command in the file.

## Open questions

1. `OnCreate`, `PostReceiveUpdate`, `PreSendUpdate`, `ApplyClientOffsetCorrection`, `SetupCollision`, `ResetDynamicMembers`, `ExecuteDestruction`, `DestroyOnePhaseLoop`, the model destruction data and the animation-event handler of the static model: **T01-175**.
2. The prop's physics (mechanism category and gravity, tool handling, net update, the update priority constants, the splash and dust effect tables): **T01-176**.
3. The values of `PropFlags`, `StaticModelAnimAction` (by order here) and the 6 low `ModelFlags` bits as numbers: run-time check **T01-184**.

## Test vectors

1. **Counts** (script): 25,383 static model entities in 59 worlds with 25,391 records (8 owned, no entity); 1,726 prop entities in 32 worlds, 1,762 records.
2. **Order rule.** `CStaticModelProperties` data members in file order `.1, .2, .3, .4, .12, .5, .6, .7, .8, .9, ep_fSoundFallOff, ...` are `ep_pmcModel, ep_vStretch, ep_smlModifiers, ep_bCollision, ep_idMechanism, ep_fCullDistance, ep_fFadeLength, ep_idParentPart, ep_idAutoStartAnim, ep_estShadows, ep_fSoundFallOff, ...`.
3. **Flags.** A static model with "Disable destruction" has `en_mfFlags` 65,536 and `IsDestructible` false; a static model with "Collision" 0 has no mechanism.
4. **Net.** A static model without destruction data and without "Control by macro" is not replicated; one with "Control by macro" (536 in the data) is `NEB_STATIC`; a prop made by a spawner is `NEB_STATIC`, a free prop `NEB_DYNAMIC`.
5. **Prop effects.** A prop touching the floor at relative speed 5.5 m/s with min 3 and max 8 spawns the collision effect at intensity (5.5 − 3) / (8 − 3) = 0.5; below 3 m/s none; at most once per 0.1 s.
