# Global parameter classes and other helper records of `Sam2Game.dll` (T01-113, part 12 of T01-026)

- **Roadmap task:** T01-113 (overview and table of all 59 classes: [`helper-classes.md`](helper-classes.md); iterators and arrays: [`helper-classes-iterators.md`](helper-classes-iterators.md); events: [`helper-classes-events.md`](helper-classes-events.md); all 932 classes: [`class-inventory.md`](class-inventory.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the member lists and counts of the 8 parameter classes that are in the data (script, section 3), for the equality of the constructors' defaults with the data's values where the page says so (`CSamNetworkParams` 8 of 8, `CBulletParameters` 5 of 5 and 8 of 9, `CObjectManipulationParams` 245 to 252 of 252 per member), for the identification of `CObjectManipulationParams` with the data's `CThrowingParams` (the defaults equal the data's values member by member, and the exported `CObjectManipulation` constructs two of them where the data has two) and for the identification of `CSamProjectParams` with the member table named `CSamTestRunParams`; **medium** for the identification of `CLPEnvironmentParams` with `SPuppetEnvironmentParams` and for the meanings that come from names; **low** for the guessed roles of the numbered environment members and the unused-class notes.

Prerequisites: [`helper-classes.md`](helper-classes.md), `rsc-ctl.md` and `rsc-ctl-schema.md` (the `.rsc` global files and every member's type and value), `ep-schema.md` (the `.ep` records), `player-rules.md` (the numbers of the global parameters), `replication.md` (the network error block), `players-session-classes.md` (game types, difficulty).

## Summary

The **global parameters** are the game's tuning constants: one `.rsc` file of the content (`GlobalGameParameters.rsc`) holds a `CGlobalGameParams` object with **80 members**, nested records, and links to the spawning parameters, the spawner tweaks and the spawner boot tweaks; the network error block (`CSamNetworkParams`) and the bullet defaults (`CBulletParameters`) are records inside it. The classes of this page have **few functions** (no named method at all except `CEntityParams`, `CLPEnvironmentParams` and `CLauncher`): they are data structures, and the exported functions are the standard seven or fewer (the plain structures have 4 to 6). **15 classes, 98 exported functions**: 11 parameter classes (71 functions), `CLauncher`, `CTextToSay`, `CToolEntityCreator` and `CFeelerSetup` (27 functions).

| Class | Data | Members | Functions | What it is |
|---|---|---|---|---|
| `CGlobalGameParams` | `CGlobalGameParams` (1 `.rsc`) | 80 | 7 | the game's tuning constants (section 1) |
| `CGlobalSpawningParams` | `CGlobalSpawningParams` (1 `.rsc`) | 8 | 7 | enemy spawning curves (section 2) |
| `CSamNetworkParams` | `CSamNetworkParams` (inside the above) | 8 | 4 | network error smoothing (section 3) |
| `CSamProjectParams` | **binary only**; member table `CSamTestRunParams` | 34 | 7 | the session / game options (section 4) |
| `COperatingParams` | `SOperatingParams` (126 `.ep`) | 4 | 6 | how a puppet is operated as a vehicle (section 5) |
| `CObjectManipulation` | `CObjectManipulation` (126 `.ep`) | 2 | 6 | the carry and throw limits of a puppet (section 6) |
| `CObjectManipulationParams` | data name `CThrowingParams` (252 values) | 6 | 6 | one set of limits (section 6) |
| `CBulletParameters` | `CBulletParameters` (inside the global parameters) | 7 | 4 | hit-scan bullet defaults and trails (section 7) |
| `CMaterialSoundRanges` | `CMaterialSoundRanges` (inside the global parameters) | 8 | 4 | how far impact sounds carry (section 7) |
| `CLPEnvironmentParams` | data name `SPuppetEnvironmentParams` (258 values, 86 `.ep`) | 9 | 7 | movement limits of one environment (section 8) |
| `CEntityParams` | `CEntityParams` (324 files) | 0 | 13 | the base of every `.ep` parameter class (section 9) |
| `CLauncher` | `SLauncher` (35 `.wld`) | 0 | 11 | a spawner's launcher (section 10) |
| `CTextToSay` | no | 3 | 6 | a queued line of speech (section 10) |
| `CToolEntityCreator` | `CToolEntityCreator` (17 `.ctl`, in its three derived classes) | 0 | 7 | what a tool becomes when thrown (section 10) |
| `CFeelerSetup` | no | not read | 3 | a plain structure without a constructor (section 10) |

**Headline findings.**

1. **Two exported classes are the data's records under other names.** `CObjectManipulationParams` is the data's `CThrowingParams` and `CLPEnvironmentParams` is the data's `SPuppetEnvironmentParams`: the constructor defaults agree with the data member by member and in order (sections 6 and 8). **High** for the first (two independent readings), **medium** for the second. This resolves the "probably" in `enemies.md` about the speed members of the environment record.
2. **`CSamProjectParams` is the class behind the member table `CSamTestRunParams`**: its constructor sets 32 of the table's 34 members at the table's own offsets and leaves the two strings empty; section 4 gives the **default value of every session option** (single player, Normal difficulty, 3 lives, 2 players, a frag limit of 20 and a time limit of 10, co-op pickups that stay). **High** for the identification (names, order and offsets agree), **medium** for each default's meaning.
3. **The network error block in the global parameters equals the class defaults exactly** (8 of 8): the shipped file keeps every default of `CSamNetworkParams` (section 3). **High**. The bullet defaults equal the file in 5 of 5 plain members and 8 of 9 trail members (the static trail's life is 0.25 in the file against 1.0 in the class).
4. **`CLPEnvironmentParams::GetInstanceMoveSpeed(t)` interpolates linearly between the third and the fourth number of the environment record** (`low + (high - low) * t`); the third is not larger than the fourth in **258 of 258** records, so the pair is a **minimum and maximum speed** and `t` a per-instance random fraction (**medium**; who calls it is open, `T01-214`).
5. **The global parameters are a registry, not code**: `CGlobalGameParams` has no method at all; the 80 members are read by the classes that need them, and section 1 links the consumers that the other parts already named.

## Evidence

- **Export table.** 59 classes of group "P11" were listed; the 15 of this page export 98 functions; names are quoted as exported.
- **Reading.** The constructors of all 15 classes were decompiled and **read** (basis **R**; they are 9 to 1,846 bytes), and the functions `CEntityParams::Copy`, `Clone`, `GetName` and `mdGetName`, `CLPEnvironmentParams::GetInstanceMoveSpeed` and the three `CLauncher` functions. Defaults were taken as the numbers the constructors store, in the member order of the data (the order was confirmed by the data's type tables: the offsets in a constructor grow in the order of the data's members).
- **Member tables.** The binary declares the member table `CSamTestRunParams` (34 members, parent `CBaseProjectParams`); its names are quoted; the offsets were used to place the constructor's stores (not recorded). The data's type tables name the members of the other classes.
- **Data.** `tools/re/helper_classes_verify.py` sections 3 and 4 read `GlobalGameParameters.rsc`, `SpawningParams.rsc` and every `.ep` for the three records and print: member counts, the nested network and bullet records against the constructors' defaults, and for `CThrowingParams` (252 instances) and `SPuppetEnvironmentParams` (258 instances) how many instances equal each default.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs. **Not checked:** the consumers of most of the 80 global members (`T01-212`), the shape and use of the six spawning graphs (`T01-213`), how the session options are written by the menu screens at run time (`menu-classes-session.md`), the whole network error smoothing (`replication.md` has the numbers).

## Layout / rules

### 1. `CGlobalGameParams` (80 members; parent `CResource`; 7 functions)

Stored in `Content/SeriousSam2/Databases/GlobalGameParameters.rsc` (the only instance; 8 objects in the file). Every number is in `rsc-ctl-schema.md` (type and value) and the player-facing ones in `player-rules.md`; this table groups the 80 members, names the consumers the other parts already found (page in brackets) and says how each meaning is known. "N" = from the member name only (medium to low); "C" = the consumer was read on the named page (medium).

| Group (members, in data order) | Value in the shipped file | Meaning and consumer |
|---|---|---|
| links: `ggp_pgspSpawningParams`, `ggp_pgstpSpawnerTweaks`, `ggp_pgsbtSpawnerBootTweaks` | references (1 each) | `CGlobalSpawningParams` (section 2), `CGlobalSpawnerTweakParams`, `CGlobalSpawnerBootTweaks` (`mechanism-classes-spawner-tweaks.md`) |
| `ggp_gppPlayerParams` (`CGlobalPlayerParams`: `gpp_fFOV`, `gpp_bAutoJumpSP`, `gpp_bAutoJumpDM`) | 80, 1, 0 | default field of view and the auto-jump options (C: `players-session-classes.md`) |
| `ggp_fDefaultInitialDelay`, `ggp_fStandardSeeDistance`, `ggp_fShortsightedDistance`, `ggp_fCharacterFOV` | 0.5, 100, 40, 360 | AI sight and start delay (N; `ai.md`) |
| `gpp_psr`, `gpp_psrOther` (`CPuppetSoundRanges`, 6 members each), `gpp_msr` (`CMaterialSoundRanges`, section 7) | records | sound fall-off and hot spot ranges of puppets (own, other) and of materials (N) |
| `ggp_fDoorSoundFallOff/HotSpot`, `ggp_fWeaponSoundFallOff/HotSpot`, `ggp_fDestructionSoundHotSpot/FallOff` | 50 / 1, 60 / 2, 5 / 75 | sound ranges of doors, weapons, destruction (N) |
| `ggp_fProjectileDefFlyingSoundFallOff/HotSpot`, `ggp_fProjectileDefExplodeSoundFallOff/HotSpot` | 30 / 1, 90 / 1 | default ranges of projectile sounds (C: `projectile-classes.md`) |
| `ggp_fProjectileThreatDistance`, `ggp_fWeaponFireThreatDistance`, `ggp_fPuppetThreatDistance` | 100, 100, 100 | distances at which a projectile, a shot, a puppet counts as a threat (C: `projectile-classes.md`, `weapon-classes.md`) |
| `ggp_ghMovePeriodOverDistance` | a graph | AI movement period over distance (N) |
| `ggp_fStayInAreaCoreRatio`, `...HysteresisRatio`, `...ShellRatio`, `ggp_fCircleAroundFoeCoreRatio` | 0.5, 0.25, 0.25, 0.5 | the stay-in-area and circle-around-foe behaviours (N; `behavior-mover-classes.md`) |
| `ggp_fLeggedPuppetLookRotation`, `ggp_fLeggedPuppetRenderableRotation` | 90, 360 | limits of the walker's look and body turn (N) |
| `ggp_fWheeledVehicleFallOutPitch/Banking`, `ggp_fWheeledVehicleCorrectionVelocity`, `ggp_fAllowedPlayerDistanceFromSeat` | 80, 80, 30, 8 | when a rider is thrown out; the allowed distance from a seat (N; `vehicles.md`) |
| `ggp_bpBulletParams` | `CBulletParameters` (section 7) | hit-scan defaults (C: `projectile-classes.md`) |
| `gpp_bEnemiesCanFightEachother`, `gpp_bAllEnemiesShouldFightEachother`, `ggp_fEnemyVSEnemyDamageMultiplier` | 1, 0, 6 | enemy against enemy (`player-rules.md`) |
| `ggp_pcscCorpseSinkConfig`, `ggp_bRenderDebrisShadow`, `ggp_fNormalDebrisBreakVelocity` | a reference, 0, 4 | corpse sinking record, debris (C: `projectile-classes.md`) |
| `ggp_fTimeToChooseLeftWeapon`, `ggp_cwuComboWeaponsUsing` | 3, 2 | the combo weapon (left hand) choice time and mode (C: `players-session-classes.md`, `weapon-classes.md`, `hud-classes.md`) |
| `gpp_dmDamageMultipliers` (`CDamageMultipliers`, 5 members) | 0.25 cutting, 1.5 explosion, 2 kicking, 6, 6 | damage-type multipliers (`player-rules.md`) |
| `gpp_bAllowMacroVariableAutoRemoval`, `gpp_fUsageDistance`, `ggp_fRayFromViewDistance` | 0, 4, 1,000 | macro variable clean-up; the use action's reach; the aim ray length (C: `players-session-classes.md`) |
| `ggp_fMinObjectShootoutTime`, `ggp_fMaxObjectShootoutTime` | 0.05, 0.5 | how long a thrown object is shot out (N) |
| `ggp_pudPowerUpDurations`, `ggp_pudPowerUpDurationsDM` | records (7 members each) | power-up durations single player, multiplayer (`player-rules.md`) |
| `ggp_snpNetworkParams` | `CSamNetworkParams` (section 3) | network error smoothing (`replication.md`) |
| `ggp_gdGameDifficulties`, `ggp_gdGameDifficultiesXBox` | records (5 levels each) | difficulty tables PC and console (`player-rules.md`) |
| `ggp_slExtraLifeScore`, `ggp_slSeriousScoreMultiplier` | 10,000, 5 | score for an extra life; the serious-score multiplier (C: `item-classes.md`) |
| `ggp_sndNoAmmoClick`, `ggp_ghAutoAimingAngle` | a sound, a graph | the empty-weapon click; the auto-aim angle curve (N) |
| `ggp_fBeaconAndObjFadeOutHotSpot/FallOff`, `ggp_fBeaconAndObjStayTime`, `ggp_fBeaconAndObjFadeOutTime` | 20, 40, 4, 3 | beacon and objective markers (`player-rules.md`) |
| `ggp_pctGenericEntityTool` | a tool reference | the tool a generic entity becomes (N) |
| `ggp_fSeriousBombGlarePeriod`, `ggp_grSeriousBombGlare` | 1, a gradient | the serious bomb's white-out (`player-rules.md`) |
| `ggp_ptxCrosshair`, `ggp_ghCrosshairSize`, `ggp_ptxNavBeaconCrosshair`, `ggp_pixNavBeaconCrosshairSize`, `ggp_ptxObjCrosshair`, `ggp_pixObjCrosshairSize` | textures; sizes 100, 100 | the three crosshairs (N; `hud-classes-special.md`) |
| `ggp_pbsExtraLife`, `ggp_pbsExtraScore`, `ggp_ptxPuppetHealthBarEdge`, `ggp_ptxPuppetHealthBarTile` | sounds, textures | extra-life and extra-score sounds; the enemy health bar (N) |
| `ggp_fWeaponItemStayTimeAfterDrop`, `ggp_fPowerUpItemStayTimeAfterDrop` | 10, 10 | how long a dropped pickup stays (C: `item-classes.md`) |
| `ggp_ffpFlamerFlameParams` | `CFlamerFlameParams` | the flame's constants (C: `projectile-classes.md`) |
| `ggp_wfGiveAllWeaponsFlags` | 49,151 | the weapon set of the "give all weapons" cheat (`player-rules.md`) |

Group sizes: 3 + 1 + 4 + 3 + 6 + 4 + 3 + 1 + 4 + 2 + 4 + 1 + 3 + 3 + 2 + 1 + 3 + 2 + 2 + 1 + 2 + 2 + 2 + 4 + 1 + 2 + 6 + 4 + 2 + 1 + 1 = **80** (the script prints the member count 80). The constructor of `CGlobalGameParams` (1,846 bytes) sets class defaults for the same members (for example the field of view 80 and the sound ranges); the file stores all 80 members, so those defaults have no effect while the file loads (`T01-212`).

### 2. `CGlobalSpawningParams` (8 members; parent `CResource`; 7 functions)

Stored in `SpawningParams.rsc`. Members: six **graphs** (`CGraphHolder`: `gsp_ghSpawnDelayOverSpawnPeriod`, `gsp_ghSpawnDelayOverEnemyHealth`, `gsp_ghEnemyMultiplierOverEnemyHealth`, `gsp_ghEnemyMultiplierOverTotalCount`, `gsp_ghGroupSizeOverEnemyHealth`, `gsp_ghGroupDelayOverEnemyHealth`) and two numbers, `gsp_iMinHealthForMaintainGroup` (**2,400** in the file against a class default of **1,000**) and `gsp_fFlyingPuppetMultiAdjustment` (**0.67**). The constructor builds a default linear graph for each of the six. The names say the spawner reads the delay and the multiplier **as functions of the enemy's health, of the total count and of the spawn period**, and the group size and delay as functions of the health; the graphs' points, units and the readers are open (`T01-213`; the spawner side is `mechanism-classes-spawners.md`). **Medium** (names).

### 3. `CSamNetworkParams` (8 members; no parent; 4 functions)

A plain record (no virtual table, 4 functions) nested in the global parameters (`ggp_snpNetworkParams`). The data's member order equals the constructor's order and **every value in the file equals the class default** (script: 8 of 8). Meanings from `replication.md` (the numbers are the same there):

| # | Member | Value | Meaning |
|---|---|---|---|
| 1 | `snp_fIgnoredPositionError` | 0.05 | a position error below this is ignored |
| 2 | `snp_fTeleportedPositionError` | 5 | an error at or above this teleports |
| 3 | `snp_fPostionErrorMeltingFactor` | 5 | the rate at which an error between the two melts away (the member name has this spelling) |
| 4 | `snp_fPredictionToleranceForTeleporting` | 0.2 | the horizontal prediction tolerance when judging a teleport |
| 5 | `snp_fYToleranceMultiplier` | 3 | the vertical tolerance is this many times that |
| 6 | `snp_fIgnoredAnimOffsetError` | 0.05 | an animation offset error below this is ignored |
| 7 | `snp_fTeleportedAnimOffsetError` | 0.5 | an animation offset error at or above this snaps |
| 8 | `snp_fAnimOffsetErrorMeltingFactor` | 5 | the melting rate of the animation offset error |

**High** for the values, **medium** for the meanings (from `replication.md`'s reading of the consumers).

### 4. `CSamProjectParams` (the session options; parent `CBaseProjectParams`; 7 functions)

Exported as `CSamProjectParams`; the DLL's member table is named **`CSamTestRunParams`** (34 members, parent `CBaseProjectParams`) and **no type table of the data names either**, so the options are never stored in a `.wld` or `.rsc` directly: they live in the **game info entity** (`en_sgpGameParams`, 28 members flagged as replicated: `replication.md`; constructed by the game info constructor), in every **stored level status** (constructed by `CStoredLevelStatus`; the options saved with a chapter, `save-game.md`) and in the **session settings screen**, whose command 12 replaces its copy by a fresh default (`menu-classes-session.md`). The constructor calls the base constructor, then sets the following (**R**, **medium** for the meaning of each default; **high** that the member and the default belong together, the constructor stores each default at the table's own offset of that member):

| Member | Default | Member | Default |
|---|---|---|---|
| `sgp_esgaStartAs` | 0 | `sgp_ctFragsLimit` | **20** |
| `sgp_strStartSaveGame`, `sgp_strStartChapterName` | empty strings | `sgp_ctTimeLimit` | **10** |
| `sgp_bContinuePlaying` | 0 | `sgp_bUseLives` | 1 |
| `sgp_bDedicatedServer` | 0 | `sgp_bRespawnInPlaceInCoop` | 1 |
| `sgp_cbtChapterBootType` | 0 | `sgp_bEnableSprinting` | 0 |
| `sgp_gtGameType` | **1** (`GT_SINGLEPLAYER`) | `sgp_iEnemyMultiplier` | **-1** |
| `sgp_gdGameDifficulty` | **3** (Normal) | `sgp_iEnemyMultiplierPerPlayer` | **-1** |
| `sgp_bFriendlyFire` | 0 | `sgp_iEnemyMaxMultiplier` | **100** |
| `sgp_iExtraEnemyStrength` | 0 | `sgp_bEnableRocketJumping` | 0 |
| `sgp_iEnemyStrengthPerPlayer` | **50** | `sgp_iRespawnInvulnerability` | **-1** |
| `sgp_ctMaxLives` | **3** | `sgp_bUseComboWeapons` | 0 |
| `sgp_ctMaxPlayers` | **2** | `sgp_iScoreForExtraLife` | **-1** |
| `sgp_ctPrivateSlots` | 0 | `sgp_fSpawnerMultiplier` | **1.0** |
| `sgp_bVoiceRequred`, `sgp_bPlayForStats`, `sgp_bInfiniteAmmo`, `sgp_bDisableSeriousBombs` | 0 each | `sgp_bWeaponStaysInCoop`, `sgp_bAmmoStaysInCoop`, `sgp_bHealthAndArmorStayInCoop` | 1 each |

The table has 34 members: the 32 numeric ones listed above plus the two strings. Reading: the default game is a **single-player game on Normal with 3 lives**; the options screen presets (`menu-classes-session.md`) override them. `-1` means "not set": the screen's lists for the enemy multipliers and the extra-life score start from values the screen writes; `players-session-classes.md` says `GetEnemyMultiplier` uses the option with a floor of 1 (**low** for the meaning of -1; `T01-217`). Enumerations: `GT_` and `GD_` values in `players-session-classes.md`.

### 5. `COperatingParams` (4 members; parent none; 6 functions)

The data's `SOperatingParams` (126 instances, one in each puppet parameter file as `pp_opOperatingParams`; constructed by the `CPuppetParams` constructor). Members in order: `op_bPreferesTurningControl` (default 0; the file has 1 in **4** of 126), `op_fCameraOriginOffsetY` (default 0; 0 in 121, **1.2 in 4**, **2 in 1**), `op_idOperatorSeat` (default the invalid identifier; `Seat` in some files, see `vehicles.md`), `op_bRequiresIgnition` (default 0; **0 in all 126**). The declared order equals the data's (script). Meanings from the names: whether the rider steers by turning rather than strafing, the camera's vertical offset while operating, the seat that operates the vehicle, whether the engine must be started first (**medium**). **Low**: nothing sets "requires ignition", so the feature is unused in the retail data.

### 6. `CObjectManipulation` and `CObjectManipulationParams` (the carry and throw limits)

- **`CObjectManipulation`** (2 members, 6 functions; data `CObjectManipulation`, 126 `.ep`, as `pp_omObjectManipulation` of every puppet parameter file): two records, `om_ompNormal` and `om_ompOnProteins`. Its constructor builds two `CObjectManipulationParams` (**R**).
- **`CObjectManipulationParams`** (6 members, 6 functions) **is the data's `CThrowingParams`** (252 instances = 2 x 126): member order and defaults agree. Members and constructor defaults: `omp_fMaxCarryingMass` **100**, `omp_fMinGrabVelocity` **5**, `omp_fMaxGrabVelocity` **20**, `omp_fMaxThrowVelocityMass` **10**, `omp_fMinThrowVelocity` **1**, `omp_fMaxThrowVelocity` **30**. The number of instances whose value equals the default (script, of 252): 245, 250, 252, 245, 252, 248 — the shipped files keep the defaults almost everywhere; the other values are 1,000, 10,000 and 201 (the two mass limits), 10 (the minimum grab velocity) and 50 (the maximum throw velocity). **High** for the identification and the member order (two independent readings: the constructors of both classes and the data), **medium** for the meanings: the heaviest object that can be carried, the speed limits of picking up and the speed range of a throw that falls with the object's mass (names only; the readers are in `CPuppetEntity::ThrowObject`, `OnObjectThrowPrepare` and `OnObjectThrow`, `puppet-entity-methods.md`).
- The second record, `om_ompOnProteins`, is the set used when the puppet is "on proteins" (a strength boost; **low**: the name is the only evidence; `T01-212`).

### 7. `CBulletParameters` and `CMaterialSoundRanges` (plain records)

- **`CBulletParameters`** (7 members, 4 functions; constructed by the `CGlobalGameParams` constructor): `bp_fDefaultBulletVelocity` 80, `bp_fDefaultBulletReach` 160, `bp_fDefaultBulletForceImpulse` 250, `bp_fBulletUpImpulseRatio` 0.5, `bp_fBulletRayDiameter` 1, then two embedded records: `bp_sbtStaticBulletTrail` (`CStaticBulletTrail`: `sbt_fLifeTime`, `sbt_fTailDelayRatio`, `sbt_fHeadDelayRatio`, `sbt_fStartWidth`, `sbt_fEndWidth`, `sbt_fTextureLength`) and `bp_mbtMovingBulletTrail` (`CMovingBulletTrail`: `mbt_fTrailLength`, `mbt_fTrailVelocity`, `mbt_fTrailWidth`). The constructor's defaults equal the file for the five plain members (5 of 5), for 5 of the 6 static trail members (life time **1.0** in the class, **0.25** in the file) and for the 3 moving trail members (script; **high**). Consumer: `projectile-classes.md` (bullets).
- **`CMaterialSoundRanges`** (8 members, 4 functions): `msr_fBulletFallOff`, `msr_fBulletHotSpot`, `msr_fDustFallOff`, `msr_fDustHotSpot`, `msr_fThrustFallOff`, `msr_fThrustHotSpot`, `msr_fSawHotSpot`, `msr_fSawFallOff` (the data's order; the last two are in this order). File: 30, 1, 20, 1, 35, 1, 1, 35. Class defaults: 20, 5, 10, 5, 30, 10, 5, 30 (**0 of 8 equal**: the file overrides all). The pairs are the distance at which an impact sound (bullet, dust, thrust, saw) is full volume (hot spot) and the distance at which it fades out (fall-off) (**medium**, names).

### 8. `CLPEnvironmentParams` = `SPuppetEnvironmentParams` (9 members; 7 functions, one named)

The record that gives the movement limits of **one environment** (land, air or water: `SLeggedPuppetParams.lpp_pepLand/Air/Water`, `puppet-legged-classes.md`). The exported class has **9 numeric members** with the constructor defaults below; the data's `SPuppetEnvironmentParams` has **9 members** stored in the order `.1`, `.2`, `.3`, `.4`, `.7`, `.5`, `.6`, `pep_fStandingRotationMin`, `pep_fStandingRotationMax` (258 instances in 86 `.ep` files, three per file). **Mapping (medium)**: the constructor's n-th default belongs to the data's n-th member in that order, because the defaults are the commonest value of the data member in every position (script):

| Position | Data member | Constructor default | Data instances equal to the default (of 258) | Role (low unless stated) |
|---|---|---|---|---|
| 1 | `.1` | 200 | 171 | acceleration-like (`enemies.md`: 40 / 40 for the Kleer) |
| 2 | `.2` | 40 | 238 | deceleration-like |
| 3 | `.3` | 6 | 195 | **minimum instance move speed** (medium: `GetInstanceMoveSpeed`) |
| 4 | `.4` | 6 | 195 | **maximum instance move speed** (medium) |
| 5 | `.7` | 2.5 | 46 (3.0 in 175) | a third, slower speed (`enemies.md`: 5 for the Kleer) |
| 6 | `.5` | 360 | 229 (an unlimited 3 x 10^38 in 4) | a rotation speed limit |
| 7 | `.6` | 360 | 229 | a second rotation speed limit |
| 8 | `pep_fStandingRotationMin` | 180 | 229 | turning on the spot, lower |
| 9 | `pep_fStandingRotationMax` | 180 | 229 | turning on the spot, upper |

**`GetInstanceMoveSpeed(t)`** (**R**, the only named method): returns `m3 + (m4 - m3) * t` (the third plus `t` times the difference of the fourth and the third). `t` is a fraction from 0 to 1 that the caller supplies per puppet instance, so identical puppets walk at slightly different speeds. The script confirms `m3 <= m4` in **258 of 258** records. `enemies.md` listed `.3`, `.4` and `.7` as the speed members (8.5 / 9.5 / 5 for the Kleer); this adds that the first two are the **range** of the instance speed (8.5 to 9.5 for the Kleer) and that their order in the record is minimum then maximum (**medium**). Who calls the method and what `t` is are open (`T01-214`).

### 9. `CEntityParams` (the base of the `.ep` classes; parent `CResource`; 13 functions)

The class every `.ep` parameter class derives from (324 files hold the class: 322 `.ep`, 2 `.wld`; it has **no members**). Named methods (**R**): `Copy(other)` copies the whole object member by member through the engine's reflection with the "retain everything" flag raised for the duration; `Clone` creates a new object of the **same dynamic class** and copies into it (so a clone of a `CKleerPuppetParams` is a `CKleerPuppetParams`); `GetName` returns the **file stem** of the resource (the file's name without folder and extension); `mdGetName` (the type system's name) is the **class name with a leading `C` and a trailing `Params` removed** (`CKleerPuppetParams` gives `KleerPuppet`); `mdPostRead` calls the class's own post-read virtual function (one of its table slots), and `mdScriptPostRead` is empty. The weapon, projectile, item, puppet and brain parameter pages derive from this class (`weapon-classes.md`, `projectile-classes.md`, `item-classes.md`, `puppet-classes.md`, `brain-classes.md`).

### 10. `CLauncher`, `CTextToSay`, `CToolEntityCreator`, `CFeelerSetup`

- **`CLauncher`** (11 functions = 7 + `BreakPropertiesLinks`, `GetLaunchVelocity`, `SetupPreview`, `UpdateLinks`; data `SLauncher`, 35 `.wld`, no members): the **base class of the launchers** that give a spawned puppet its start velocity (`CMarkersLauncher` and `CRelativeRandomConeLauncher` in `projectile-classes.md`). In the base class `SetupPreview`, `BreakPropertiesLinks` and `UpdateLinks` are **empty** (one shared 1-byte function; `UpdateLinks` is the same function as `BreakPropertiesLinks`) and `GetLaunchVelocity` stores a **constant vector held in the DLL** into its output argument. Users that refer to the class: `CSpawnerEntity::OnUpdate`, `AdjustForLauncherAndFormationParams` and `PrintSpawnerMultiplierInfo` (**P**; `mechanism-classes-spawners.md`).
- **`CTextToSay`** (6 functions; not in the data): one **queued line of speech** of the HUD conversation element: three members, the **talker** (the `Talker` enumeration: player, puppet or Netricsa), the **text** (a string) and a **duration** (float). `CConversationHudElement::Say(talker, text, duration)` appends one to its list `che_attsTextsToSay` (`hud-classes-special.md`). **Medium**.
- **`CToolEntityCreator`** (7 functions; no members, no named method; data: 17 `.ctl`): the **base class of the tool entity creators**, the objects that say what entity a tool becomes when it is thrown or dropped (`ct_ptecToolEntityCreator` of `CCharacterTool`, `rsc-ctl.md`). The data has three derived classes (**not exported**): `CDebrisToolEntityCreator` (`dtec_fLifeTime`, `dtec_bExplodeOnTimer`, `dtec_pfxTrail`; 7 files), `CBaseItemToolEntityCreator` (`bitec_pepBaseItemProperties`: the pickup's properties; 8 files) and `CGenericProjectileToolEntityCreator` (`gptec_pgppProjectileParams`; 2 files): 7 + 8 + 2 = 17. **High** for the counts, **medium** for the roles (`rsc-ctl.md`).
- **`CFeelerSetup`** (3 functions, **no constructor**): only the three operator functions of a plain structure are exported; no member table, no data. The "feeler" members of the puppet (`en_fFeeler...`, `puppet-entity-classes.md`) are separate. Everything beyond the name is open (`T01-214`).

## Invariants

1. `CGlobalGameParams` has 80 members in the data's type table and no named method (script, 1 file).
2. The class defaults of `CSamNetworkParams` equal the file's values: 8 of 8; of `CBulletParameters`: 5 of 5 plain members, 5 of 6 static trail members and 3 of 3 moving trail members.
3. `CThrowingParams` has 6 members and 252 instances in 126 files, exactly two per file; for each member between 245 and 252 of the 252 values equal the constructor default of `CObjectManipulationParams` in the same position.
4. `SPuppetEnvironmentParams` has 9 members and 258 instances in 86 files, three per file; the third member is not larger than the fourth in 258 of 258.
5. `SOperatingParams`: `op_bRequiresIgnition` is 0 in 126 of 126 instances.
6. The three tool entity creators of the data add up to the 17 files of their base class (7 + 8 + 2).

## Open questions

1. **Consumers of the 80 global members** not already named in other parts, and the meaning of `om_ompOnProteins`: **T01-212**.
2. **The six spawning graphs**: points, units, readers: **T01-213**.
3. **The environment record's roles** (the first two and the rotation members), the users of `GetInstanceMoveSpeed` and the meaning of `CFeelerSetup`: **T01-214**.
4. **The meaning of the session options' -1 values** and the writers and readers of each option: **T01-217** (with the menu parts).

## Test vectors

1. `GetInstanceMoveSpeed` of a Kleer land record with `.3` = 8.5 and `.4` = 9.5: `t` = 0 gives 8.5, `t` = 0.5 gives 9.0, `t` = 1 gives 9.5.
2. A freshly constructed `CSamProjectParams`: game type 1, difficulty 3, 3 lives, 2 players, frag limit 20, time limit 10, `sgp_iEnemyMaxMultiplier` 100 and four options at -1.
3. `CEntityParams::mdGetName` of an object of class `CKleerPuppetParams`: `KleerPuppet`; `GetName` of a resource loaded from a file whose path ends in `Kleer.ep` (an example name): `Kleer`.
