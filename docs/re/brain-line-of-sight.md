# The line-of-sight service of the brain (T01-244)

- **Roadmap task:** T01-244 (follow-up of [`brain-senses.md`](brain-senses.md) open questions 1 and 3; the foe manager's numbers are [`foe-manager-numbers.md`](foe-manager-numbers.md), T01-156)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the enumeration values and the member names of the request (read from the DLL's own tables) and for the data counts of `tools/re/line_of_sight_verify.py`; **medium** for every rule read from a function (the request fill-in, the ray set-up, the "seen" test, the sound ray test, the turret override), each one reading with the instruction listing checked for the argument order; **medium-low** for the eye-point rule (several members identified only by their position in a table, marked) and for the effect of the avatar; **low** for what the collision category `character_sees_character` lets through (not read). **Nothing was run.**

Prerequisites: [`brain-senses.md`](brain-senses.md) (who asks), [`foe-manager-numbers.md`](foe-manager-numbers.md) (when), [`puppet-params-loading.md`](puppet-params-loading.md) (the parameter files), [`hitscan-shot.md`](hitscan-shot.md) (the engine's ray calls are shared with shooting).

## Summary

The brain's "can A see B" is a **ray cast with a request object**: a `CLOSRequest` (the DLL's name) is filled in, executed at once, and its answer is **delivered inside the same call**, to a receiver entity when one is set. There is no queue, no budget per frame and no range of its own. What an implementer needs:

1. **The request** (section 1): who from, who to (or which point or direction), the **purpose** (`LRP_LOOK` 0 for seeing, `LRP_SHOOT` 1 for aiming), the collision **category** of the ray, a ray **radius** (0 in every brain request), the receiver, and the result (hit entity, hit point, normal, distance, and `lrq_bIsEntitySeen`).
2. **Execution is synchronous** (section 2): the fill-in functions compute the two end points, cast one ray of that exact length (no range cap), store the hit and **call the receiver's `OnSeeTarget` or `OnNotSeeTarget` before returning**. The caller may then read the result fields; the foe manager relies on both.
3. **"Seen"** (section 2): for an entity-to-entity request: the ray hits **nothing**, or its **first hit belongs to the target entity**. Anything else (a wall, another entity) is "not seen". Only the kind "entity to entity" can say seen after a hit; the other kinds say seen only when nothing is hit.
4. **End points of a puppet-to-puppet request** (section 3): from the asker's **eye point** (`GetViewOrigin`: its placement plus the body parameter `pbp_vViewOrigin` times the puppet's stretch, rotated by the placement, with pose and sitting variants) to the target's **view target** (`GetViewTarget`: the model's origin plus `pbp_vViewTarget` times the stretch, rotated by the look direction). In the data the common eye height is 1.5 (102 of 124 files) times the stretch.
5. **The sound ray test** (section 4): after a heard sound the hearer casts one ray from its eye point to the emitter's **threat-sound origin** (an eye point for puppets, the placement for other entities) with the same category; the hearer walks to the sound when **nothing is hit or the first hit is the emitter entity**.
6. **Turrets ignore all threat sounds** (section 5): the turret class overrides the sound-check question with "no". The scream, the eye point of a sound and the ranges are settled in the sibling page and in section 5.

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Read (decompiled; the argument order of every call checked in the instruction listing): the five request fill-in functions and the ray set-up and result functions behind them (unnamed in the DLL, described by their roles), `CFoeManagerEntity::LookForFoe`, `TrackCurrentFoe`, `TrySwitchingToAnotherFoe`, `ThreatSoundHeard`, `OnSeeTarget`; `CPuppetEntity::CanSeeFoe`, `GetViewOrigin`, `GetViewTarget`, `GetRelativeViewOrigin`, `GetThreatSoundOrigin`, `ShouldExamineThreatSound`, `GetPuppetStretch`; `CLeggedPuppetEntity::GetRelativeViewOriginForPose`; `CBaseEntity::GetThreatSoundOrigin`, `GetViewOrigin`, `GetViewTarget`; `CTurretPuppetEntity::ShouldExamineThreatSound`. The request's member names (`lrq_*`) and the enumerations `LOSRequestPurpose`, `LOSRequestType` and `EPuppetPose` were read from the DLL's member and enumeration tables (names and values; the member offsets agree with the order used by the functions and are not recorded here, `conventions.md`). The engine's own ray calls (`rayInit`, `raySetRay`, `raySetMaxDistance`, `raySetRayRadius`, `cldSetAvatar`, `cldSetRayCategory`, `cldCheckRay` ...) are imported by name from `Engine.dll`; their bodies were **not** read (T01-220).
- **Data.** `python -I tools/re/line_of_sight_verify.py "<install>"` (about 1 second): section 7.
- **Not checked.** Nothing was run. What the category `character_sees_character` collides with (the engine's category table), the exact effect of the avatar entity, the hit-entity rule of `cldGetHitAspect` and the player's view animation were not read. Callers other than the foe manager (`CacheBestMarker` of a marker mover, `CanSeeFoe` users such as the attack preconditions of `behavior-generic-puppet.md`) were identified, not read.

## Layout / rules

### 1. The request (`CLOSRequest`; names from the DLL, **high**; meanings **medium**)

| Member | Type | Meaning |
|---|---|---|
| `lrq_tmRequestTime` | time | the simulation time the request was filled in |
| `lrq_henFromEntity`, `lrq_henToEntity` | entity handles | the asking and the looked-at entity (empty for point and direction kinds) |
| `lrq_henToNotify` | entity handle | the **receiver** of the answer (empty: no callback) |
| `lrq_henHitEntity` | entity handle | the entity that owns the first thing the ray hit (empty when nothing was hit) |
| `lrq_lrpRequestPurpose` | `LOSRequestPurpose` | `LRP_INVALID` -1 (default), `LRP_LOOK` 0, `LRP_SHOOT` 1 |
| `lrq_vFromPoint`, `lrq_vToPoint` | `Vector3f` | the two end points (filled from the entities or given) |
| `lrq_vDirection` | `Vector3f` | the unit direction of the ray (the length of the segment is kept separately in the call) |
| `lrq_vHitPoint`, `lrq_vHitNormal`, `lrq_fHitDistance` | | the hit point, its surface normal and the distance from the start |
| `lrq_psmHitMaterial`, `lrq_phlHitHull` | pointers | the surface material and the hull that was hit |
| `lrw_lrtRequestType` | `LOSRequestType` | `LRT_ENTITY_TO_ENTITY` 0, `LRT_POINT_TO_POINT` 1, `LRT_ENTITY_DIRECTIONAL` 2, `LRT_POINT_DIRECTIONAL` 3, `LRT_INVALID` -1 (default) |
| `lrq_bIsEntitySeen` | bool | the answer of section 2 |
| `lrq_idCategory`, `lrq_idThickCategory` | identifiers | the collision category of the ray and of its thick version; **invalid means "bullet"** (the shooters' category, `hitscan-shot.md`) |
| `lrq_fRayRadius` | float | the ray radius (0 by default and in every request of the brain) |
| `lrq_hmechAvatar` | mechanism handle | an optional mechanism to treat as the asker's own body (null in every request read) |

A new request starts as: type and purpose invalid, categories invalid, radius 0, all handles empty, points and direction zero.

**The five fill-in functions** (each also sets the time to now and the two category identifiers; all then run the ray of section 2):

| Kind | Arguments | Sets |
|---|---|---|
| entity to entity (type 0) | asker, target, purpose, category, thick category | from-point = the asker's `GetViewOrigin(purpose)` position, to-point = the target's `GetViewTarget(purpose)`; both entities stored; segment length = the distance between the points |
| point to point (type 1) | two points, categories | the points; no entities |
| entity directional (type 2) | asker, a direction, purpose, categories | from-point = the asker's `GetViewOrigin`; **length 3.0e38** (unlimited) |
| point directional (type 3) | a point, a direction, categories | length 3.0e38 |
| **entity to point** (type left **invalid**) | asker, a point, purpose, categories | from-point = the asker's `GetViewOrigin(purpose)`, to-point = the given point; the asker entity is **not** stored, so this ray has **no avatar entity** |

The brain and the foe manager use **entity to entity** (looking at a puppet) and **entity to point** (the sound ray test). `CPuppetEntity::CanSeeFoe(purpose)` is entity to entity from the puppet to its own foe puppet with **no receiver**; it uses the category `character_sees_character` for `LRP_LOOK` and the default ("bullet") for `LRP_SHOOT`, and returns `lrq_bIsEntitySeen`.

### 2. Execution and the answer (**medium**)

For every kind the same steps run, in this order:

1. The segment from the from-point to the to-point has length `L` (types 0 to 1 and the entity-to-point kind) or 3.0e38 (directional kinds). The direction is the segment normalised; **a zero-length segment gives a zero direction** (no division).
2. One **ray** of that direction and length: minimum distance 0, maximum `L`, radius `lrq_fRayRadius`, with the asker entity as the collision system's **avatar** (the entity whose own body the ray is allowed to ignore; the effect is the engine's, **medium-low**) and the mechanism avatar, the **category** (or "bullet" when invalid) and the thick category (or "bullet"); then the engine's ray check runs.
3. The result is copied into the request: hit distance, hit point, normal, material, the hull that was hit, and the **entity that owns that hull** (`lrq_henHitEntity`; empty when nothing was hit or the hull has no owner).
4. **Seen.** `lrq_bIsEntitySeen` := **true** when (a) the ray hit nothing, or (b) the request type is `LRT_ENTITY_TO_ENTITY` and the hit entity is the request's target entity. Otherwise **false** (a hit on a hull that has no owning entity is therefore "not seen"). (The entity-to-point kind has type invalid, so after a hit it is always false; its callers read the hit entity instead, section 4.)
5. **Answer.** When `lrq_henToNotify` is a live entity: seen calls its **`OnSeeTarget(request)`**, not seen calls its **`OnNotSeeTarget(request)`**; both are virtual functions of the base entity (empty there; the foe manager overrides them, `foe-manager-numbers.md` 2.3). All of this happens **before the fill-in call returns**.
6. A debug option can record the request in a ring buffer (not specified).

The service has **no range of its own and no limit on the number of requests**: the callers apply their own limits before asking (the see distance and the field of view in `LookForFoe`, `brain-senses.md` 3 and 4; the radius of the sound in `samEmanateThreatSound`, `brain-senses.md` 5.1).

### 3. The eye point and the view target of a puppet (**medium-low**; the parameter members are identified by their table positions)

- **Stretch.** `GetPuppetStretch` = the parameter `pbp_fStretch` times the puppet's own replicated stretch member (`en_fStretch`), and 1.0 without a parameter file.
- **Relative eye point** (`GetRelativeViewOrigin(purpose)`, the vector part): `pbp_vViewOrigin` times the stretch; when the riding state `en_rsRidingState` is 3 (sitting in a ride) the sitting variant `pbp_vViewOriginSitting` instead. **Legged puppets** (`GetRelativeViewOriginForPose`, the puppet's current pose): when not sitting, the **y component** is replaced by `pbp_fViewHeightCrouch` for `PP_CROUCH` (2), by `pbp_fViewHeightDive` for `PP_DIVE` (3) and by `pbp_fViewHeightSwim` for `PP_SWIM` (4); the x and z components are those of `pbp_vViewOrigin`; everything is multiplied by the stretch. Other poses (`PP_STAND` 1 and the rest) keep `pbp_vViewOrigin`. Without a parameter file the vector is a constant default.
- **Eye point** (`GetViewOrigin(purpose)`): the puppet's placement position plus the **relative eye point rotated by the placement's orientation**. For `LRP_SHOOT` the puppet's shooting-origin attachment (a point on the model, `GetShootingOriginAttachment`) or `pbp_qvShootOrigin` replaces the eye point; that is the shooters' origin (`hitscan-shot.md`) and not used by the brain. The returned orientation (the look direction) is not used by the request.
- **View target** (`GetViewTarget(purpose)`): `LRP_LOOK`: the model renderable's position plus **the stretch times a parameter vector rotated by the puppet's look direction**; `LRP_SHOOT`: the renderable's position plus the stretch times the shoot-target vector, **not rotated**. For puppets that are *looked at* the look vector is the body parameter I call `pbp_vViewTarget`; the shoot vector is `pbp_vShootTarget`.
- **Threat-sound origin** (`GetThreatSoundOrigin`): a **puppet's** is the position of its `GetViewOrigin(LRP_LOOK)` (the eye point); every **other entity's** is its placement position (`CBaseEntity::GetThreatSoundOrigin`). So the distance of a sound to a hearer (`brain-senses.md` 5.2 step 4) is measured from the hearer's **placement** to the emitter's **eye point** (correcting `brain-senses.md` 5.1, "the position of a puppet is its position"); the emitter list of `samEmanateThreatSound` measures from the emitter's **placement**.

Data (section 7): in the 124 puppet files the stretch ranges from 0.5 to 30 (1.0 in 45 files), the standing eye height `y` of the first vector member of the body record is **1.5 in 102 files**, the sitting variant 0.8 in 118, and the three pose heights of the 84 legged puppets are **the same constants everywhere: crouch 0.7, swim 0.4, dive 0.0** (the player: stretch 0.9, standing eye 1.95). The values are in the model's own units; the multiplication by the stretch gives metres of the world (**medium-low**: the unit of the model is not stated here).

### 4. The sound ray test (`ThreatSoundHeard`; **medium**)

Used when the emitter has **no brain** (a weapon fired by a non-puppet, an alarm: `brain-senses.md` 5.2 step 5) and when the emitter has a brain that the hearer **could not see**. With `P` the hearer's puppet and `E` the emitter entity:

1. The hearer's threat sensitivity 7 skips the test: the puppet walks to the sound at once (`ExamineSoundSource`), or for a brain emitter takes it as a `FH_HEARD` foe.
2. Otherwise **entity to point** (section 1) from `P` to `E`'s threat-sound origin, purpose `LRP_LOOK`, category `character_sees_character`, thick category default, **no receiver**.
3. **Result:** when the **hit entity is empty** (nothing was hit) **or equals `E`**, the hearer calls `ExamineSoundSource(E's threat-sound origin)` (it walks to that point unless its allowed moving range is 0 or more, `brain-senses.md` 5.3); in every other case it does nothing.
4. For a **brain emitter** `B` the first step is an ordinary entity-to-entity request (section 1) from `P` to `B`'s puppet with no receiver and **purpose `LRP_LOOK`**; when it says seen, the hearer takes `B` as a `FH_SEEN` foe; when not, step 2 runs with `E` the emitter entity. (For the scream `B` is the screamer's foe and `E` the screamer: `foe-manager-numbers.md` 2.3.)

### 5. Turrets, ranges, the random member, the projectile distance

- **The turret's override.** `CTurretPuppetEntity::ShouldExamineThreatSound` returns **false** (a three-byte function that returns zero, shared with many other "return false" functions; **medium**): **a turret puppet ignores every threat sound**. The base puppet's version returns "the sound check is not disabled" (`en_bDisableThreatSoundCheck` false, `brain-senses.md` 5.2 step 1).
- **Which `.cb` member is which range:** `cb_fActiveRange`, `cb_fSenseRange`, `cb_fThreatRange` are, **by their member offsets** (not by declared order), the three ranges used as named in `foe-manager-numbers.md` 2.2 (**medium**).
- **The per-puppet random value** `en_fRnd` of the scream rule is **uniform in [0, 1]** (a 32-bit random integer times 2^-32, drawn once in the constructor; 0 for player puppets): `foe-manager-numbers.md` 6.
- **`ggp_fProjectileThreatDistance`** (100 in the data) is **not read by any function of `Sam2Game.dll`** that reads the global game parameters (`foe-manager-numbers.md` 8): unused at run time.

### 6. Who asks (**medium**)

`LookForFoe` and `TrackCurrentFoe` (entity to entity, receiver = the foe manager), `TrySwitchingToAnotherFoe` (entity to entity, no receiver, result read at once), `ThreatSoundHeard` (entity to entity without receiver, and entity to point), `CPuppetEntity::CanSeeFoe`, and `CacheBestMarker` of the marker movers (entity to entity; the marker choice is T01-155 and T01-242). The shooters' and projectiles' rays (`FireBullets`, `DoDamage`, feelers) fill the request object only for the shared reset and use their own ray set-ups.

### 7. Data (script `tools/re/line_of_sight_verify.py`; **high** for the numbers, **medium-low** for the member identification)

From the 124 puppet `.ep` files:

- `pbp_fStretch`: 0.5 to 30 in 30 distinct values (1.0 in 45, 0.75 in 14, 2.0 in 8).
- First vector member of the body record (`pbp_vViewOrigin`, identified by position): x and z are **0 in all 124**; y is **1.5 in 102**, 0 in 4, 3.5 in 4, 3.0 in 3, and 1 to 2 files each for 0.3, 1.75, 1.95, 2.0, 3.8, 4.0, 5.0, 6.0, 10.0.
- Sitting variant: 0.8 in 118, 1.2 in 2, 0 in 2, 0.2 and 4.0 in 1 each.
- The other two vector members (the view target and the shoot target; which is which is not decided by the data): the view-target candidate equals the eye point in **102** files; the two take the values 1.5 in 98 and 96 files; the extremes are -2 and 80.
- Legged puppets with the three pose heights: **84**, all crouch 0.7, swim 0.4, dive 0.0.

## Invariants

1. A request answered through a receiver is answered before the fill-in call returns (the only two request sites that set a receiver, `LookForFoe` and `TrackCurrentFoe`, both read).
2. `lrq_bIsEntitySeen` is true for a hit only when the request type is entity-to-entity and the hit entity is the target.
3. The category of every request of the foe manager and the sound code is `character_sees_character` with purpose `LRP_LOOK` and the thick category default (the 6 request sites of `LookForFoe`, `TrackCurrentFoe`, `TrySwitchingToAnotherFoe` and `ThreatSoundHeard`, argument order checked in the listing).
4. A puppet's eye point is above its placement by `y x stretch` where `y` is 1.5 for 102 of the 124 parameter files when standing.

## Open questions

1. **What the category `character_sees_character` collides with** (which entities and surfaces block it; whether glass and the sky box do; the table that assigns the categories) and what the avatar does: the engine's collision (`Engine.dll`, T01-220 for the ray cast; **T01-362** for the category table of the game).
2. **The exact `GetViewTarget` and `GetViewOrigin` rules** (which parameter member is the view target and which the shoot target; the model unit; the player's view animation; what the origin does for vehicles and aircraft): **T01-363**.
3. **`CacheBestMarker`** and the other users of the service (marker movers): T01-155, T01-242.
4. **Run-time check** that a puppet with a clear line sees a puppet 1 m behind a thin wall (the ray-length test): T01-130, T01-038.

## Test vectors

- **Seen:** two standing puppets 40 apart with nothing between them: the ray reaches the target point: **seen**. A pillar between them: the first hit is the pillar: **not seen**. A third puppet in the line: the first hit belongs to it: **not seen**. The ray ends exactly at the target's body: the first hit is the target: **seen**.
- **Eye point:** a standing puppet with stretch 0.9 and `pbp_vViewOrigin` y 1.95 (the player): the ray starts 1.755 above its placement; the same puppet crouching on a legged body with crouch height 0.7: 0.63 above. A chicken (stretch 2.5, y 0.3): 0.75.
- **Sound ray test:** hearer at the origin, emitter entity (a fired weapon of a non-puppet) at 30 in front with a wall between: the ray hits the wall: the hearer does nothing; the same without the wall: the ray hits nothing: the hearer walks to the emitter's placement (unless it has an allowed moving range). Sensitivity 7: it walks without any ray.
- **Turret:** the same sound: the turret does nothing (`ShouldExamineThreatSound` is false).
- `python -I tools/re/line_of_sight_verify.py "<install>"` prints the stretch, eye-height and pose-height tables of section 7.
