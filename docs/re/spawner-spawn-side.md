# Spawner spawn side: placement, formations, launchers, effects and templates (T01-165, part 2)

- **Roadmap task:** T01-165. Part 1 (states, procedures, delays, the world info protocol): [`spawner-state-machine.md`](spawner-state-machine.md). Counters and events: [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md). Launcher internals: [`projectile-classes.md`](projectile-classes.md) section 8.2.
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the member tables and values of the `.sfc` files, launchers, formations, multi-spawners and spawner records (every instance of 59 worlds and the 11 external files, script `tools/re/spawner_spawn_verify.py`); **medium** for every placement rule, the order of the spawn steps and the template choice (one reading of the code each, arguments recovered from the instruction listing, formation and perturbation formulas checked only against the member tables); **low** for the spawn-source puppet side and the points marked "open". **Nothing was run**: no placement was observed.

Prerequisites: [`spawner-state-machine.md`](spawner-state-machine.md), [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md), `wld-navigation.md` (formation markers), `puppet-classes.md` (the spawnee side), `conventions.md`.

## Summary

Every spawn passes through the same pipeline (part 1 state 3, or `SpawnOne`, or `DeathSpawn`):

1. **Origin** = the spawner's placement, replaced by the **formation's** placement for this spawnee (section 3);
2. **Velocity** = the **launcher's** velocity (section 4) or the spawner's **random cone** (section 5);
3. **Spawn effect** (section 6): with no configuration the spawnee is made at once; with a configuration an **effect entity** plays and a **spawn delayer** makes the spawnee after the configured delay (or a **projectile** carries it, section 7); with a **spawn source** the source puppet plays animations and releases the spawnee (section 8);
4. **`SpawnOneSpawnee`** (section 9) copies the template record, makes the entity at an **upright** placement, tells it who spawned it, raises `SpawneeAvailable` and launches it.

Key numbers: the spawn origin is raised **0.1 m** before an "auto fall down" search; the cone launcher's default pitch is 90 degrees, its default speed 5 to 10 or 10 to 15 m/s; a formation picks its slot with an index that is a permutation derived from `ep_fRndBase` and a **random table that is different on every start of the game** (section 10).

## Evidence

- **Code.** `SpawnSourceAndSpawnEffect`, `SpawnReady`, `DoSpawnEffect`, `FinishConfigurationSpawning`, `SpawnOneSpawnee`, `LaunchEntity`, `GetRandomSpawnVelocity`, `DeathSpawn`, `SetSpawnProperties`, `CSpawnDelayerEntity::OnCreate` and `SpawnIt`, `CMultiSpawnerProperties::GetPropertiesToSpawn`, `UpdateSpawnTemplates`, `GetSpawneeCountLimitType`, the placement functions of the five formation classes and of the weighted marker list (not exported: found through the callers of the weighted list and of the formation call site), the two launchers, `rtGetPerturbedIndex` and `rtInitializeRandomTables` of `Core.dll`, the spawn-source functions and `SetSpawner` of the puppet class were decompiled; every lost argument was read in the instruction listing. Constants were read as floats from the data section.
- **Data.** `python -I tools/re/spawner_spawn_verify.py` (about 10 minutes): section 1 the 11 `.sfc` files, section 2 the 3,783 spawner records, 3 the launchers, 4 the formations, 5 the multi-spawners. Counts below are from that run.
- **Not checked.** The spawnee side beyond `SetSpawner`, the source puppet's movement, the weighted marker list's editor code, the `CSpawnMarkerEntity` role, the numeric values of the animation identifiers (`IDENT` values 0 to 3 of the `.sfc` files), the real random table values (they differ per run).

## Layout / rules

### 1. Where the spawn comes from

`SpawnSourceAndSpawnEffect` (state 3, part 1) is the normal path; `SpawnOne` (type 5) has its own copy of the origin and velocity code in `CommonEntryPoint`; `DeathSpawn` sets origin and velocity directly. The spawner has its origin in `en_qvSpawnOrigin` (28 bytes: a quaternion and a position), its velocity in `en_vSpawnVelocity`, and the arguments below come from these members: `en_ctSpawned` (index of the spawnee, from 0), `en_ctTotal`, `en_ctGroup` (clamped to at least 1 and **written back** by `SpawnSourceAndSpawnEffect`), `en_iMultiplier`, `en_bGroupSpawner`, `ep_fRndBase`/`en_fRndBase` ("RND", 1,755 distinct values in (0, 1)), `en_fLaunchPitch`.

### 2. `SpawnSourceAndSpawnEffect`, step by step (read; medium)

Let *k* = `en_ctSpawned`, *g* = `en_ctGroup` (at least 1), *j* = *k* mod *g*, *m* = `en_iMultiplier`. The placement arguments **(index, count)** used in steps 2 and 3 are:

| Case | index | count |
|---|---|---|
| *m* above 1 | *j* | *g* |
| *m* at most 1, `en_bGroupSpawner` 0 | *k* | `en_ctTotal` |
| *m* at most 1, `en_bGroupSpawner` 1 | *k* | *g* (formation) / `en_ctTotal` (launcher) |

(The group-spawner distinction applies to the **formation** call only; the launcher always gets `en_ctTotal` when *m* is at most 1.)

1. `en_ctGroup` := *g*.
2. **Formation** (`en_hfFormation`, data: 2,488 spawners without, 1,295 with): the closest player to the spawner is found (`samGetClosestPlayer`, any distance); `en_qvSpawnOrigin` := the formation's placement for (closest player, RND, current origin, index, count) (section 3).
3. `en_fLaunchPitch` := **0.7853982 rad (45 degrees)**, every time.
4. **Launcher** (`en_hlcLauncher`, 690 spawners): `en_vSpawnVelocity` := the launcher's velocity for (the spawner, the current origin, `en_fLaunchPitch`, RND, index, count) (section 4). If the launcher is a **markers launcher**, additionally the **rotation part** of the origin is replaced by the rotation of the marker the launcher's weighted list picks for (RND, index, count) (the position is kept); the marker is the same one that the launch targets.
5. **Spawn source** (`en_henSpawnSource`, 7 spawners): if there is one and a spawn effect configuration exists: tell the source to start (section 8) and enter **state 1** (part 1); the rest of the spawn happens in `SpawnReady`. If there is a source but **no configuration**: log "Spawner that uses spawn source doesn't have spawn effect configuration defined!" and continue as without a source (the data has 1 such spawner of 7).
6. **No source** (or the error above): if `en_bUseRandomCone`, `GetRandomSpawnVelocity` (section 5) **overwrites** the launcher's velocity.
7. `DoSpawnEffect` (section 6), then `CommonEntryPoint` again (part 1).

**`SpawnReady(placement)`** (virtual; called by the source puppet at the animation event `ReleaseSpawnee`, or by `ManuallyReleaseSpawnee`): ignored unless the state is 1; `en_qvSpawnOrigin` := the given placement; if there is a launcher the velocity := the launcher's velocity for (spawner, new origin, `en_fLaunchPitch`, RND, index = `en_ctSpawned`, count = `en_ctTotal`) (note: always the total, regardless of *m*); then `DoSpawnEffect` and `CommonEntryPoint`. The random cone is **not** applied on this path.

### 3. Formations: `CFormation` and its five classes (read; medium)

A formation answers one question: **where is spawnee *index* of *count*?** The virtual call is `Placement(closestPlayer, RND, basePlacement, index, count)` and returns a quaternion and a position. All classes first reduce the index: **i := index mod count**; if flag 1 (`CFormation.1`) is set, **i := count - 1 - i**; for the line, rectangle, V and pie slice: if flag 2 (`CFormation.2`) is set, **i := PERTURB(RND, i, count)** (section 10). The five flags of the base class `CFormation` (`CFormation.1` to `.5`, five BOOLs; `.6` an INDEX: **kind number 1 line, 2 V, 3 rectangle, 4 pie slice, 5 markers**, present only in the newer layout: 1,274 + 474 + 148 + 50 + 36 = 1,982 of 2,730 objects) are, in the data:

| Flag | Meaning (code) | Values in the data |
|---|---|---|
| `.1` | reverse the order | 0 in all |
| `.2` | permute the index (all classes but markers: the markers class permutes inside its marker list, flag `.2` of that list) | 1 in at least 80 markers, 44 line, 18 rectangle and 16 pie slice formation objects; 0 in the other combinations printed |
| `.3` | add a fixed local offset (two static vectors, zero in the DLL image) | 1 in at least 18 pie slices, never in the other classes |
| `.4` | not read by any placement function | 1 in all |
| `.5` | **random**: ignore the index, take fresh random numbers (the global generator, not the table) | 1 in at least 24 pie slices, never in the other classes |

If the offset flag `.3` is set an extra rotated offset is added whose source vectors are zero in the image of the DLL and may be set at run time (**low**: no code that sets them was found). The result of every class is: **orientation = the base placement's orientation unchanged**; **position = base position + R(base orientation) times the local offset** below (local x to the right, y up, **forward is -z**). `RND` is `ep_fRndBase`.

- **`CLineFormation`** (member `.1` the length L; 638 objects, L from 2 to 148): s = (count = 1 ? 0.5 : i / (count - 1)) (or a fresh random in [0, 1) with flag `.5`); offset = (**(s - 0.5) * L**, 0, 0). The first spawnee is at the left end, the last at the right end, centred on the base.
- **`CVFormation`** (68 objects; `.1` length L, `.2` opening angle A in degrees: 120 to 320 in the data): s as for the line; half-angle a = A/2 degrees. For s below 0.5: offset = **(1 - 2s) * L * (-sin a, 0, -cos a)**; otherwise **(2s - 1) * L * (+sin a, 0, -cos a)**: a V whose apex is at the base placement and whose arms of length L open **forwards** (-z); the first spawnee is at the end of the left arm.
- **`CPieSliceFormation`** (`.1` inner radius r1, `.2` outer radius r2, `.3` angle W in degrees: 30 to 360; 68 objects): s as for the line (count 1: 0.5; random flag: two fresh randoms, one for s, one for the radius); radius = r1 + (r2 - r1) * u, where **u is a table value** (section 10: the entry with index `round(1023 * RND) + i`, taken modulo 1024); angle = (s - 0.5) * W degrees; offset = (**sin(angle) * radius**, 0, **-cos(angle) * radius**): spawnees on an arc symmetric about the forward axis, spread from left to right.
- **`CRectFormation`** (`.1` to `.4`, four floats: rectangle A has width `.1` (x) and depth `.3` (z), rectangle B width `.2` and depth `.4`; 196 objects): s = i / count (not count - 1; or a fresh random); a point **P_A** on the perimeter of rectangle A at the parameter s and a point **P_B** on rectangle B; offset = **P_A + u * (P_B - P_A)** with u the table value as for the pie slice (y = 0). The perimeter walk (`Perim(s, depth, width)`, centre at the origin) is piecewise with the breakpoints **0.125, 0.375, 0.625, 0.875**: s below 0.125: (x, z) = (-4 s * width, +depth/2); 0.125 to 0.375: (-width/2, depth/2 - 4 (s - 0.125) * depth); 0.375 to 0.625: (-width/2 + 4 (s - 0.375) * width, -depth/2); 0.625 to 0.875: (+width/2, -depth/2 + 4 (s - 0.625) * depth); from 0.875: (width/2 - 4 (s - 0.875) * width, +depth/2). The walk starts in the middle of the rear edge (z = +depth/2; forward is -z), first towards -x, and visits the four corners at 0.125, 0.375, 0.625 and 0.875; each quarter of the parameter belongs to one edge regardless of the edge's length, so the **spacing along the edge depends on its length**.
- **`CMarkersFormation`** (1,760 objects; one `CWeightedFormationMarkers` list): i as above (no permutation here); the list's placement (below). The list holds `CWeightedFormationMarker` items (each: a marker entity, the marker's record, a weight; 5,252 items in the formations: 2,311 name a `CFormationMarkerEntity`, 2,311 name a `CFormationMarkerProperties` record (the saved copy) and 630 are empty; **weight 1 in 5,246 of 5,252, 2 in 2, 5 in 4**); the list has two flags: **`.2` "randomise"** (set in 94 of 1,760 formations, 50 of 332 launchers) and **`wfm_bClosestToPlayer`** (0 in all formations, 1 in 26 launchers).
  - **Pick** (`PickItem(RND, index, count)`): W := the sum of the weights of the items whose marker entity exists; n := min(W, count), at least 1; slot := index mod n; with the randomise flag slot := PERTURB(RND, slot, n); the picked item is the first whose running weight sum exceeds slot (ranges of consecutive slots per item; a weight of 2 gets two consecutive slots); if there is no valid item the base placement is returned unchanged. **When count is smaller than W only the first `count` slots are reachable.**
  - **Placement of the pick**: the marker entity's absolute placement; **if the marker has its own formation** (`CFormationMarkerEntity` member `.2`: 70 markers, `wld-navigation.md`) the spawnees that picked this marker are spread by that formation: its placement is asked for (closest player, RND, **the marker's placement as the base**, rank, group), where **group** is the number of indices below `count` that picked the same marker and **rank** the position of this index among them.
  - **Closest to player** (`wfm_bClosestToPlayer` with a closest player present): the candidates are every marker's own placement and the placements of all indices 0 to count - 1 computed as above; the one whose position is **nearest to the closest player** is returned (the same one for every index).

The shapes are tested with the vectors of section 11.

### 4. Launchers, from the spawner's side (read; medium)

`en_hlcLauncher` is a `CLauncher`: `SRelativeRandomConeLauncher` (524 spawners) or `SMarkersLauncher` (166). Both implement `GetLaunchVelocity(spawner, origin, pitch, RND, index, count)`. **Spawner side** (this page): the call arguments of section 2; the result is stored as `en_vSpawnVelocity` and used by `SpawnOneSpawnee` (section 9). **Launcher side** (`projectile-classes.md` section 8.2): the cone launcher takes a random heading within +-`rcl_fAngleH` / 2, a pitch in [`rcl_fMinPitch`, `rcl_fMaxPitch`] and a speed in [`rcl_fMinVelocity`, `rcl_fMaxVelocity`], all from **one random table entry** (index `round(1023 * RND) + index`) and turned by the origin's orientation; the markers launcher solves a free-fall throw to the picked marker with the given pitch and multiplies the throw by `ml_fLaunchPower`. Data (counts of launcher objects in the files; `.1` to `.5` are the cone's members): minimum pitch 90 degrees in 864, maximum pitch 90 in 874 (45, 55, 60, 70 ...), angle 0 in 928, minimum speed 5 in 716 (10 in 188), maximum speed 10 in 484 (15 in 388); the markers launcher: launch power 1 in 298 (0.5 in 18), `ml_bCTPFlagConverted` 1 in all 332 (the old "closest to player" flag was converted into the list's flag, 26 set), the pitch graph null in all, marker lists of 1 to 35 items. Spawner records with a markers launcher and a formation: 17 (the launcher's marker rotation then replaces the formation's); cone launcher and a formation: 24; a launcher and no formation: 649; a formation and no launcher: 1,254 (`launcher x formation` table of the script).

### 5. The random cone (`GetRandomSpawnVelocity`; read; medium)

Used when `en_bUseRandomCone` (set by the multiplier code and tweaks, cleared by `ResetSpawner`; defaults in `mechanism-classes-spawners.md` section 2: 15, 45, 90, 10, 15): take the random table entry `T` at index `round(1023 * RND) + en_ctSpawned` (three values r1, r2, r3 in [0, 1)); heading offset = (r1 - 0.5) * `en_fAngleH` degrees; pitch = `en_fMinPitch` + (`en_fMaxPitch` - `en_fMinPitch`) * r2 degrees; speed = `en_fMinVelocity` + (`en_fMaxVelocity` - `en_fMinVelocity`) * r3; the velocity is the **forward axis** (-z, in the frame of the Euler rotation (heading, pitch, 0) composed with the spawner origin's orientation) times the speed. The origin's own orientation therefore points the cone.

### 6. `DoSpawnEffect` (read; medium) and the spawn effect configuration

**Orientation.** The "effect placement" has the origin's position and an orientation whose forward axis is the **normalised launch velocity** (when its length exceeds 0.00001) or else the origin's own forward axis; heading = atan2(-x, -z) (0 when the direction is vertical), pitch = asin(y), bank 0. If `en_bAutoFallDown`: the position is raised by **0.1 m** along the effect placement's up axis and a fall-down search (`samGetFallDownPosition`) may replace it by the ground position below (used for the effect entity only; section 9 does its own search for the spawnee).

**Branches** by the configuration `en_sfcSpawnEffectConfig` (`ep_sfcSpawnEffectConfig`, an inline `CSpawnEffectConfiguration` or an external `.sfc` file):

1. **No configuration** (2,361 of 3,783 spawners): `SpawnOneSpawnee(en_qvSpawnOrigin, index = en_ctSpawned, en_vSpawnVelocity, launch = 1)` at once.
2. **Configuration with no effect and no sound** (`sfxc_pefEffect` and `sfxc_pbsSound` both null; 50 inline records and the 4 uses of the shared `SpawnCapsule` file in the data): `FinishConfigurationSpawning(effect placement, en_vSpawnVelocity, en_ctSpawned)` at once (section 7).
3. **Configuration with an effect or a sound** (the rest): a **`CSimpleEffectProperties`** entity is created at the (fall-down adjusted) effect placement carrying `sfxc_pefEffect`, `sfxc_fEffectStretch`, `sfxc_fEffectPower` and `sfxc_pbsSound`; then a **`CSpawnDelayerProperties`** entity at the unadjusted effect placement holding `sfx_fSpawnDelay`, the spawner, the spawnee index (`en_ctSpawned`, stored as a float) and the velocity.
4. In all branches `DoSpawnEffect` ends with **`CountOneSpawned`** (part 1 section 8): **the spawnee is counted when the effect starts, not when the spawnee appears**; a delayed spawnee (up to 2.2 s later) is already counted in `OneSpawned`, `GroupSpawned` and `AllSpawned`.

**`CSpawnDelayerEntity`** (a run-time helper; no data object): on creation it schedules its function `SpawnIt` after **max(`sfx_fSpawnDelay`, 0.00001) s**; `SpawnIt` calls the spawner's `FinishConfigurationSpawning(placement, velocity, round(index))`.

**`CSpawnEffectConfiguration`** (15 members; the binary's names; script section 1 prints all 11 external files):

| Member | Meaning (code; data) |
|---|---|
| `sfxc_fSpawnSourceStopChance` | probability that the **spawn source puppet stops moving** before spawning (0 in 729 inline, 1 in 57, 0.25 in 38; 0.25 in `spawncapsule` and `spawnsphere`, 1 in `comet` and the zombie-agent file) |
| `sfxc_bShouldMatchThrowDir` | 1 in `comet`, `spawncapsule`, `spawnsphere`; use not found (low) |
| `sfxc_idSpawnSourcePrepareAnimation` | animation the source plays first (0 in every file: none) |
| `sfxc_idSpawnSourceAnimation` | the source's spawn animation (1 in five files; the anim event `ReleaseSpawnee` of it releases the spawnee) |
| `sfxc_idNoAnimAttachment` | attachment of the source used as origin by `SpawnOne` (the identifier `Barrel01` when unset; 0 in every file) |
| `sfxc_pefEffect`, `sfxc_fEffectStretch`, `sfxc_fEffectPower` | the effect, 1 and 1 in every file |
| `sfxc_pbsSound` | sound |
| `sfx_fSpawnDelay` | seconds between the effect and the spawnee: inline **0.9 in 254, 0 in 223, 0.5 in 157, 0.1 in 118, 2.2 in 47, 1 in 25**; files: materialisation 0.9, other dimension 0.5, soul collector 2.2, thunder 0.1, comet 0.1, spawn capsule 1, the rest 0 |
| `sfxc_idProjectile`, `sfxc_pgppProjectileParams`, `sfxc_fProjectileLifeAfterTouch`, `sfxc_fProjectileStretch` | the carrying projectile (section 7): its parameter file (`SpawnComet`, `SpawnCapsule`, `SpawnSphere`, `SpawnVomitZombieAgent`), time to live after the first touch (-1, 0.01, 1, 2 s), stretch 1 |
| `sfxc_idSpawneeAnimation` | animation the spawnee plays when it is created (`SetSpawner` of the puppet; 1 in the shared "appear" files, 2 in `comet`; identifier numbers, names not recovered) |

External files and use (the 7 named in the data, 588 uses + 824 inline): `Materialisation` 340, `OtherDimension` 86, `Ground` 85, `Thunder` 36, `Comet` 31, `SoulCollector` 6, `SpawnCapsule` 4 (the 10 records that name other files use the remaining external files of the 11). Inline configurations by branch: effect or sound and a directly made spawnee **670**, effect or sound and a carrying projectile **104**, no effect and no sound with a projectile **35**, with neither **15**.

### 7. `FinishConfigurationSpawning` (read; medium)

If the configuration has **no projectile parameter file**: `SpawnOneSpawnee(placement, index, velocity, launch = 1)` (the spawnee appears at the effect's end). Otherwise a **`CGenericProjectileProperties`** entity is created at the placement: parameter file from `sfxc_pgppProjectileParams`, stretch `sfxc_fProjectileStretch`, life after touch `sfxc_fProjectileLifeAfterTouch`, power 1, **speed = the length of the launch velocity**, owner = the spawn source if it is a puppet, **spawner = this spawner**, and the spawnee index. When that projectile explodes (`CBaseProjectileEntity::OnExplode`) it calls the spawner's `SpawnOneSpawnee` at its own position with the orientation turned **towards the closest player** (heading atan2, pitch asin, no bank; the projectile's own orientation when no player exists), the stored index and its launch parameters: the spawnee is delivered by the capsule or comet.

### 8. The spawn source puppet (read in part; low)

The spawner tells the source "you will be my spawn source" (`YouWillBeMySpawnSource`) at `SetSpawnSource` and calls its `DoSpawnSource` at each spawn. `DoSpawnSource` rolls a random number against `sfxc_fSpawnSourceStopChance`: if the roll is **not above** the chance the source **stops** (waits speed / deceleration seconds, at least 0.00001) then `SpawnSourceStopped`, otherwise it proceeds at once. The source plays `sfxc_idSpawnSourcePrepareAnimation` (when set: waits its length), then `sfxc_idSpawnSourceAnimation` and calls `SpawnReady` (section 2) at the anim event `ReleaseSpawnee`; at the end of the animation the source resumes its brain. When the source puppet dies, is deleted or is shut down it calls `OnSpawnSourceDied` of its exploiting spawner (part 1 section 11). This is the puppet's half; its movement states belong to `puppet-entity-classes.md` (**T01-304**).

### 9. `SpawnOneSpawnee(placement, index, velocity, launch)` (read; medium)

1. **Pick the template.** If the template record (`en_hepSpawnProperties`) is a `CMultiSpawnerProperties`: the chosen template's record is copied (`epCopyEntityPropertiesWithLinks`) after `GetPropertiesToSpawn(index, en_ctTotal)` (section 10); else the spawner's own template is used directly.
2. **Upright placement.** The given quaternion is converted to Euler angles and **the two angles other than the heading are set to 0**: the spawnee is always created **upright, facing the origin's heading**. For a **non-multi** template the position's y is raised by **`en_iAddToY` metres** (int; the "increase Y" tweak; 0 in all stored spawners); the multi-spawner path does not add it.
3. The record receives the placement (`SetPlacement`), its spawner and multi-spawner links are broken (`BreakMultiSpawnerLinks` and one further hook), then **`SpawnEntity`** creates the entity; if the template copy was made it is deleted. If creation fails: log "Spawner::SpawnOneSpawnee(): Entity can't be spawned even though spawn properties are valid (%1)." and run `CancelSpawningInProgress` (part 1 section 5).
4. If the entity is a **puppet** and the limit type is 3: the puppet's "time spawned" := now.
5. The entity's `SetSpawner(spawner)` is called (puppets: stores the spawner handle for kill and pick reports and plays `sfxc_idSpawneeAnimation` of the configuration, if set and the model has it).
6. `en_henLastSpawned` := the entity; if `en_henSpawnerExploiter` is set, that spawner's **spawn source := the new entity**; the world info's last spawner := this spawner (`GetLastSpawned`, `level-logic.md`); the event **`SpawneeAvailable`** is raised on the spawner (512 script handlers in 41 worlds).
7. Unless `en_bDisableSpawnSound` (1,764 of 3,783 set it): a one-shot `CSimpleEffectProperties` entity plays the spawn sound (`SpawnEffect.wav`, `en_pbsSpawneeDelivered`) at the spawn.
8. If `launch` (always 1 in the call sites found): if `en_bAutoFallDown` the position is raised **0.1 m** along the up axis and `samGetFallDownPosition` may move the entity to the ground below (717 spawners); then `LaunchEntity(entity, velocity, angular)`: the entity's linear velocity := the stored launch velocity (the angular-velocity argument was not recovered). Without a launcher or cone the velocity is zero.
9. `CountOneSpawned` follows in the caller (part 1 section 8): **the spawned counter and `OneSpawned` / `GroupSpawned` / `AllSpawned` come after `SpawneeAvailable`**.

### 10. The multi-spawner's template choice, the perturbation and the random table

**Template choice** (`CMultiSpawnerProperties::GetPropertiesToSpawn(index, count)`, with `ep_fRndBase` and `ep_bRandomizeTemplateOrder`): the provider holds a list of `CWeightedSpawnTemplate` (each: a template entity, a copy of its record, an integer **weight**; 120 + 120 records in 15 worlds: **2 templates in 80, 3 in 14, 4 in 14, 1 in 12**; weights 1 to 40, total weight 6 in 20, 8 in 18, 9 in 10 ...). W := the sum of the weights of templates whose entity exists **and is marked as a multi-spawner template** (otherwise logged "Multi spawner uses template entity that is not marked as multi spawner template." and skipped); n := min(W, count) (at least 1); slot := index mod n; with the randomise flag (set in **114 of 120** records) slot := PERTURB(RND, slot, n); the template is the one whose weight range (templates in list order, consecutive ranges) contains slot. `SpawnOneSpawnee` passes index = `en_ctSpawned` and count = `en_ctTotal` (**after** the multiplier: more spawnees do not change the proportions, they only repeat the cycle of n slots; with count below W only the first templates are used). `UpdateSpawnTemplates` (run when the template list changes): each template's record is re-copied; **all templates must have the same spawnee count limit type** (else "Multi spawner can't use entities that have different spawnee limiting behavior" and the template is dropped); `GetSpawneeCountLimitType` returns the limit type of the first valid template (3 in 58 multi-spawners, 4 in 2).

**`PERTURB(seed, i, n)`** (`rtGetPerturbedIndex`; read; medium): build the list 0..n-1; let b := round(1023 * seed); for k = 0 .. n-2: r := the **first float** of the random table entry number `(b + k) mod 1024`; j := min(k + 1 + round(r * (n - k)), n - 1); swap the list's entries k and j. Result: **list[i]**. So the permutation depends on (seed, n) only; **every call recomputes it**.

**The random table** (`rtInitializeRandomTables`, run once when the engine starts): 1,024 entries of three floats, each a uniform fraction in [0, 1) (a 32-bit pseudo-random integer divided by 2^32), produced by two linear generators whose state at start is `Seed1 = low word of the CPU time-stamp counter * 0x48C27395` and `Seed2 = low word XOR 0x12345678`. **The table is therefore different on every start of the game**: nothing in the formation, launcher, cone or template choice that uses it is reproducible between runs; within one run the result for a given spawner and index is fixed. (`rtGetCountOfRandomTableEntries` returns 1023, the table is indexed with `& 1023`.) The generator state is also used directly for the `.5` "random" formation flag and the spawn source's stop roll.

### 11. Test vectors (from the code; no run)

1. **Line.** L = 10, count 5, no flags: offsets x = -5, -2.5, 0, 2.5, 5 (index 0..4); count 1: x = 0.
2. **V.** L = 10, A = 90 (a = 45 degrees), count 5, s = 0, 0.25, 0.5, 0.75, 1: offsets (x, z) = (-7.07, -7.07), (-3.54, -3.54), (0, 0), (3.54, -3.54), (7.07, -7.07).
3. **Pie slice.** r1 = 2, r2 = 2 (fixed radius), W = 120, count 3: s = 0, 0.5, 1: angles -60, 0, +60 degrees: (x, z) = (-1.73, -1.0), (0, -2), (1.73, -1.0).
4. **Rectangle perimeter.** A = 4 wide (x) by 2 deep (z), B equal to A (no ring), count 8, i = 0..7 (s = 0, 0.125, ... 0.875): (0, +1) at s 0; s 0.125: (-2, +1) (corner); 0.25: (-2, 0); 0.375: (-2, -1); 0.5: (0, -1); 0.625: (+2, -1); 0.75: (+2, 0); 0.875: (+2, +1) (position given by the formula above with width 4 and depth 2).
5. **Weighted templates.** weights (2, 1), count 6, no randomise: indices 0..5 give templates A, A, B, A, A, B (n = min(3, 6) = 3; slot = index mod 3: 0, 1 -> A, 2 -> B). With count 2: n = 2, slots 0, 1 -> **A, A** (B is never reached).
6. **Markers.** a list of 2 items with weights 1 and 2, count 6: n = min(3, 6) = 3; indices 0..5 pick items 0, 1, 1, 0, 1, 1; the marker of item 1 with its own V formation gets group size 4 and ranks 0..3 for indices 1, 2, 4, 5.
7. **Effect timing.** `Materialisation` (delay 0.9): spawnee i appears **0.9 s after** the state-3 step that counted it; `OneSpawned` fires 0.9 s earlier than the entity exists.

## Invariants

1. The 15 members of `CSpawnEffectConfiguration` have one layout in all 11 `.sfc` files and all inline objects (script section 1).
2. Every `CFormation` object has the five flags (`.4` is 1 in all) and, in the newer layout, the kind number matching its class (1 line, 2 V, 3 rectangle, 4 pie slice, 5 markers: 1,982 of 1,982).
3. A multi-spawner's template weights are positive integers; 0 total weight is not present (the minimum 1).
4. A marker item never has both its entity and its record set in the data (2,311 items with the entity only, 2,311 with the record only, 630 empty).

## Open questions

1. **The spawn source puppet's half** (movement, `ShouldMatchThrowDir`, `idNoAnimAttachment`), the numeric values of the animation identifiers. **T01-304.**
2. **Table-based randomness in a run**: nothing deterministic can be checked until a harness exists; whether any shipped mode seeds the generator (the seed is time-based in the code read). T01-174.
3. **Static offset vectors** of the formation flag `.3` and the "closest to player" tie rule. **T01-305.**
4. **`CSpawnMarkerEntity`** (177 in 16 worlds): no link from formations or launchers to it was found; formations use `CFormationMarkerEntity`. **T01-305.**
5. The angular-velocity argument of `LaunchEntity`, the exact lag of the spawn sound entity. T01-174.
