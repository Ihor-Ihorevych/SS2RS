# Foe and sense rules of the brain (T01-129)

- **Roadmap task:** T01-129 (follow-up of [`brain-classes.md`](brain-classes.md) open question 5; the foe manager's numbers are T01-156)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** only for what `tools/re/brain_senses_verify.py` reproduces from the data (the three ranges and the alignment of the 82 `.cb` files, the stored threat sensitivities and instance overrides of the placed puppets, the alignment table of section 7); **medium** for every rule read from the binary (the registration, the foe search, the sound rules, the field of view and see distance), because each rests on one reading of a function that was decompiled once; **medium-low** where a member is identified only by its position in a table (marked) and **low** for "(name)" and the open questions. **Nothing was run.**

Prerequisites: [`brain-classes.md`](brain-classes.md), [`behavior-entity-classes.md`](behavior-entity-classes.md) (foe manager, `FoeHostility`, section 7), [`brain-assignment.md`](brain-assignment.md) (which puppets have a foe manager), [`brain-movement.md`](brain-movement.md), [`puppet-classes.md`](puppet-classes.md) (the `TS_` enumeration, section 3.3), [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md), `ai.md`.

## Summary

A brain with a foe manager (a puppet with a `.cb`, `brain-assignment.md` 4) **looks for a foe**, **tracks** it, and **reacts to sounds**. The work is split over four objects:

| Object | What it does |
|---|---|
| **The puppet** | owns the senses' settings: the **threat sensitivity** (1 to 8), the optional per-instance **field of view**, **short-sight distance** and **sense range**, the **allowed moving range**, the **sound-check switch**, the alignment, the facing |
| **The brain** | answers two questions for the foe manager: `GetSeeDistance` (how far it sees) and `GetFoeSeekingFOVDeg` (the field of view in degrees); and reacts to sound with `ExamineSoundSource` |
| **The foe manager** (`CFoeManagerEntity`, one per brain with a `.cb`) | the search and the tracking loops, the line-of-sight requests, the sound rule `ThreatSoundHeard`, the hostility ranking |
| **The world info entity** | the list of existing puppets (the candidates), the list of brains that currently need a foe (the ones that hear threat sounds), the "may be foes" test and its alignment rule, the registration of rides |

Findings an implementer needs first:

1. **`OnCreatedByPuppet` registers a ride, not a brain** (corrects `brain-classes.md` section 3.1 once more, already corrected by T01-125): when the puppet has seats the brain calls the world info's `RegisterRide(puppet)`; `OnDelete` calls `UnregisterRide`. The **list of brains without a foe** (`en_eaBrainsWithoutFoe`) is a different list, kept by `BrainNeedsFoe` and `BrainDoesntNeedFoe` of the world info entity: the foe manager puts the brain in it when it **starts looking for a foe** and takes it out when a foe is assigned. **Only brains in that list hear threat sounds** (section 5).
2. **The eight threat sensitivities** (`EThreatSensitivity`) decide everything about the senses (section 2): 1 and 2 never look for a foe, 3 only "senses" a foe that is near, 4 is short-sighted, 5 is the standard, 6 sees without limit, 7 also hears through walls, 8 sees through walls.
3. **See distance** (section 3): the global standard distance (100 in the data); with sensitivity 4 the puppet's own short-sight distance when it is not negative, else the global short-sighted distance (40); with sensitivity 6 the constant **3.0e38** (unlimited).
4. **Field of view** (section 3): the **full angle** in degrees; a candidate is rejected when the cosine of its angle from the facing is below the cosine of half the field of view. 360 degrees (the global value and the only value in the data) rejects nothing; 360 is also used while the brain examines a sound.
5. **Foe search** (section 4): candidates come from the world info in cyclic order; a candidate farther than the behavior's **active range** is ignored; nearer than the **sense range** it is assigned as a **`FH_SENSED`** foe; otherwise it must be in the field of view and nearer than the see distance and a **line-of-sight request** decides: a sighting assigns `FH_SEEN`.
6. **Sound** (section 5): emitters send a threat sound with a radius (weapon fire, a puppet's death, a puppet that has had its foe for a while, an alarm entity); every brain in the "needs a foe" list within the radius runs `ThreatSoundHeard`: it is ignored by puppets with sensitivity 3 or lower, with the sound check disabled, with a patrol or navigation mover running, and (sensitivity 4) beyond the short-sight distance; a brain emitter becomes a foe (`FH_HEARD` when the hearer hears through walls, `FH_SEEN` when it can be seen); a non-brain emitter is **examined**: the puppet **walks to the sound's position** (state `BS_EXAMINE_THREAT_SOUND`) unless its allowed moving range is not negative.
7. **Alignment** (section 7): `CanBeEnemyToMyAlignment` is **not symmetric** for the semi-neutral alignment.

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Read in full (decompiled once, with the callees named): `CBrainEntity::OnCreatedByPuppet`, `OnDelete`, `GetSeeDistance`, `GetFoeSeekingFOVDeg`, `ExamineSoundSource`, `OnFoeAssigned`, `OnGoalPointReached`, `HasFoe`, `AssignFoe`; `CFoeManagerEntity::LookForFoe`, `TrackCurrentFoe`, `ThreatSoundHeard`, `OnSeeTarget`, `OnNotSeeTarget`, `BeginLookingForFoe`/`AssignFoeNow` (outline), `CalculateSleepTime` (outline: its numbers are T01-156); `CWorldInfoEntity::RegisterRide`, `UnregisterRide`, `BrainNeedsFoe`, `BrainDoesntNeedFoe`, `CharacterBorn`, `GetPossibleFoe`, `DoIHavePossibleFoe`, `GetFoeCandidates`, `CanBecomeMyEnemy`, `CanBeEnemyToMyAlignment`; `CPuppetEntity::ShouldExamineThreatSound`, `GetThreatSoundOrigin`, `IsRidable`; `samEmanateThreatSound`, `CAlarmEntity::EmanateThreatSound`, and the two weapon `OnFire` functions that call it. Member identities come from the byte offsets compared with the DLL's member tables (names and order only: offsets are not recorded, `conventions.md`); virtual calls were identified by their position in the virtual tables of the classes involved (identical shared stubs are not used as evidence). The five float constants quoted (0.5, 0.017453292, 3.0e38, 360, 0) were read from the data section.
- **Callers.** Every function of the DLL was scanned for direct virtual calls through the table positions of `ExamineSoundSource`, `GetSeeDistance`, `GetFoeSeekingFOVDeg`, `ShouldExamineThreatSound`, `OnFoeAssigned` and `OnFoeLost`: `ExamineSoundSource` is called by `ThreatSoundHeard` (3 places; two more hits use the same table position on other classes' objects); `GetFoeSeekingFOVDeg` only by `LookForFoe` (and the debug drawing of the world info entity); `GetSeeDistance` by `LookForFoe`, `ThreatSoundHeard`, one marker mover and a few non-brain functions (table position shared with other classes).
- **Data.** `python -I tools/re/brain_senses_verify.py "<install>"` (the `.cb` part 1 minute, the worlds about 20) prints sections 1 to 3 quoted in section 6.
- **Not checked.** Nothing was run. `CalculateSleepTime` and the periods of the two loops (T01-156), the line-of-sight service itself (the request object and its answer; T01-244), the member of the `.cb` that each of the three ranges is (identified by declared order only), the hostility graph branch of `CanBecomeMyEnemy`, who calls `HuntFoe` and `ForceFoe`, were not read in full.

## Layout / rules

### 1. Registration and who is in which list (**medium**)

- **World info lists** (`CWorldInfoEntity`, all empty in the files, filled at run time): `en_eaPuppetsInExisting` (every puppet that booted: `CharacterBorn`, called from the first block of the puppet's `OnBoot`, `brain-assignment.md` 1; the candidate list of the foe search), `en_eaEnemiesAlive` (also filled by `CharacterBorn`, only for an evil puppet, alignment 1, whose health is below 8,000), `en_eaBrainsWithoutFoe` (`BrainNeedsFoe(brain)` adds it unless present, `BrainDoesntNeedFoe` removes it) and `en_eaRidesInExisting` (`RegisterRide`).
- **`OnCreatedByPuppet`** (the part that concerns this page): if `HasFoeManager`, spawn the foe manager (`brain-classes.md` 3.1); set the default puppet; if the puppet is ridable, `RegisterRide(puppet)`; enter the puppet's default `.cb` state. It does **not** touch the list of brains without a foe: that is the foe manager's job (below).
- **Foe manager and the list.** The foe manager starts in `FMS_LOOKING_FOR_FOE`; `BeginLookingForFoe` tells the world info entity that the brain needs a foe (`BrainNeedsFoe`); `AssignFoeNow` (foe assigned) tells it the brain has one (`BrainDoesntNeedFoe`) and goes to `FMS_TRACKING_CURRENT_FOE`; losing the foe goes back. So **the list holds exactly the brains whose foe manager is looking**, and a brain that is tracking a foe does not hear threat sounds.

### 2. Threat sensitivity (`EThreatSensitivity`, member `en_tsCurrentThreatSensitivity`; **medium**)

The puppet's default value (`en_tsDefaultThreatSensitivity`, placed `.1300`, set by the spawner tweak `CSpawnerTweakChangeThreatSensitivity` and the macro `macSetThreatSensitivity`) is copied into the current one when the brain is created (`brain-assignment.md` 2 step 4). Effects, by value:

| Value | Name | Search (section 4) | See distance (3) | Sounds (5) |
|---|---|---|---|---|
| 1 | `TS_DEADHORSE` | never looks for a foe | standard | ignored |
| 2 | `TS_DAMAGEONLY` | never looks for a foe | standard | ignored |
| 3 | `TS_DAMAGEANDSENSE` | assigns a foe only when it is **nearer than the sense range** (`FH_SENSED`); never by sight | standard | ignored |
| 4 | `TS_SHORTSIGHTED` | standard rules | **short-sight distance** | heard only within the short-sight distance |
| 5 | `TS_STANDARD` | standard rules | standard | heard |
| 6 | `TS_LONGSIGHTED` | standard rules | **3.0e38** (unlimited) | heard |
| 7 | `TS_HEARTHROUGHWALLS` | standard rules | standard | heard **through walls** (no line-of-sight test) |
| 8 | `TS_SEETHROUGHWALLS` | **assigns the candidate at once as `FH_SEEN`** (no field of view, sense range, see distance or line-of-sight test), subject only to the active range | standard | heard |

Placed data (script section 2, below): the stored default is 1 to 8; the run-time current value is 5 in every stored puppet (it is written when the brain is created).

### 3. See distance and field of view (**medium**; constants read)

**`GetSeeDistance`** (the question the foe manager asks, in world units):

1. Start with the global `ggp_fStandardSeeDistance` (**100** in the data).
2. With no puppet, return it.
3. If the current threat sensitivity is **4**: when the puppet's `en_fInstanceShortSightDistance` is **negative** (below 0.0) use the global `ggp_fShortsightedDistance` (**40** in the data), else the instance value.
4. If the current threat sensitivity is **6**: return **3.0e38** (a float that no distance reaches).
5. Otherwise the value from step 1.

**`GetFoeSeekingFOVDeg`** (degrees, the **full** opening angle):

1. While the brain's state is `BS_EXAMINE_THREAT_SOUND`: **360**.
2. Else when the puppet's `en_fFOVInstanceOverride` is **0 or more**: that value.
3. Else the global `ggp_fCharacterFOV` (**360** in the data).

**The test** (in `LookForFoe`, section 4): let `u` be the unit vector from the brain's position to the candidate and `f` the puppet's facing (its orientation's local forward axis; the axis is computed from the quaternion of the brain's placement, which is the puppet's); the candidate **fails** the field of view when `dot(u, f) < cos(FOV * 0.5 * pi / 180)`. The constants are 0.5 and 0.017453292 (pi/180). With FOV 360 the cosine is -1 and nothing fails; with 90 degrees the half angle is 45 degrees.

The data (script section 2): the global values are the standard 100, short-sighted 40 and field of view 360 (`rsc-ctl.md`); the per-instance FOV override is stored as -1 except in a few puppets (5 distinct values, -1 to 360).

### 4. The foe search (`CFoeManagerEntity::LookForFoe`, state `FMS_LOOKING_FOR_FOE`; **medium**)

Each call, in order:

1. **Player priority.** When `en_bAutoChangesFoes` is false, the puppet's alignment is **evil** (1) and `en_lafLastAssignedFoe` is 1, try `SwitchToClosestPlayer`; when it succeeds, the manager changes state and the call ends. (`en_lafLastAssignedFoe` 1 is probably "the last foe was a player", **not resolved**, T01-156.)
2. **Pick a candidate** with the world info's `GetPossibleFoe(brain, ...)`: scan `en_eaPuppetsInExisting` **cyclically**, starting after the previously tested candidate (`en_henLastTestedFoe`) when it is in the list, else at a **random** index; take the **first** puppet that is a puppet, is not this brain's own puppet, and passes `CanBecomeMyEnemy(my puppet, it, 0)` (section 7). `en_bHasFoeCandidates` := a candidate was found; `en_henLastTestedFoe` := it. No candidate: go to 8.
3. **Threat sensitivity below 3** (1, 2): go to 8 (no search).
4. Let `d` be the **3-D** distance from the brain to the candidate's puppet. Ranges: the **active range** `A` and the **sense range** `S` of the puppet's `.cb` (the first two of its three range members in declared order: `cb_fActiveRange`, `cb_fSenseRange`; **medium-low**, by order); when the puppet's `en_fInstanceSenseRange` is **0 or more** it replaces `S`. When `d >= A` (**not** `d > A`): go to 8 (a candidate beyond the active range is ignored).
5. **Sensitivity 8:** `AssignFoe(candidate, FH_SEEN, -1)`; when the manager has left the looking state the call ends, else (the assignment was refused by its rank rules) continue at the see-distance test of step 6 (the field of view test is skipped).
6. **Otherwise:** when `d < S`: `AssignFoe(candidate, FH_SENSED, -1)` (the foe manager's `AssignFoe` has its own rank rules, `behavior-entity-classes.md` 7.3); when the manager has left the looking state the call ends, else continue at the see-distance test below (the field of view test is skipped). When `d >= S`: **sensitivity 3:** go to 8; then the field of view test (section 3): a failing candidate goes to 8. **See distance** (every path that gets here): `d < GetSeeDistance()` is required, else go to 8.
7. **Line-of-sight request** from this brain's puppet to the candidate's puppet, with the foe manager as the answer's receiver; the answer arrives later as `OnSeeTarget` (the target is seen: **in the looking state: `AssignFoe(that puppet's brain, FH_SEEN, -1)`**) or `OnNotSeeTarget` (clears `en_bFoeSeenOnLastTest`). `OnSeeTarget` also records `en_tmFoeLastSeenTime`, `en_vFoeLastSeenPos` and sets `en_bFoeSeenOnLastTest` when a foe exists.
8. **Sleep and repeat.** With no puppet: the next period is `CalculateSleepTime`. Otherwise if the "auto-assign" time has passed (`en_tmAutoAssignFoe`, a request that `AssignFoe` stores) the candidate `en_henToBeFoe` is assigned now (`AssignFoeNow`) and the call ends; else the next call is after the **smallest period 1e-5 s** (the engine's minimum, the same as in `brain-movement.md` 5.2). (Whether the period in the common case is `CalculateSleepTime` is T01-156: this reading shows 1e-5 s only on the path above.)

### 5. Sound (**medium**)

**5.1 Emitters** (`samEmanateThreatSound(emitter brain, emitter entity, radius)`): for every brain in `en_eaBrainsWithoutFoe` whose position is **closer than the radius** (strictly less) to the emitter entity's threat-sound position, call that brain's foe manager's `ThreatSoundHeard(emitter brain, emitter entity)`. The emitters read:

| Emitter | Brain argument | Radius member | Value in the data |
|---|---|---|---|
| a weapon firing (`OnFire` of the base weapon and of the shooter entity) | the shooter's brain | `ggp_fWeaponFireThreatDistance` | 100 |
| `OnShouldDie` of a brain (a puppet dying) | the dying brain | probably `ggp_fPuppetThreatDistance` (**medium-low**: by the member's name) | 100 |
| `TrackCurrentFoe` (a puppet that has had its foe for a while: section 5.4) | its brain | probably the same puppet distance | 100 |
| `CAlarmEntity::EmanateThreatSound` | none (empty) | the alarm entity's own member | (a world value) |

`ggp_fProjectileThreatDistance` (100) has no direct caller of `samEmanateThreatSound` in this build (it may be used by a path that was not found; T01-244). The threat-sound position of a puppet is its position (`GetThreatSoundOrigin`).

**5.2 `ThreatSoundHeard(emitter brain B, emitter entity E)`**, in order (my puppet is `P`):

1. Ignore unless `P.ShouldExamineThreatSound()` is true: that is `en_bDisableThreatSoundCheck` is **false** (the turret class overrides this function; its body was not read, T01-244).
2. Ignore unless `P`'s current threat sensitivity is **4 or more**.
3. Ignore when the brain has a current mover and that mover `CanOperateWithoutTargetEntity` (a **marker** or **navigation** mover: a patrolling or navigating character does not stop for sounds).
4. `d` := 3-D distance from `P` to E's threat-sound position. With sensitivity **4** ignore when `GetSeeDistance() < d` (so a short-sighted puppet hears only within its short-sight distance).
5. **E has no brain** (a weapon fired by a non-puppet, an alarm): with sensitivity **7**, `ExamineSoundSource(position)`; otherwise a ray test from `P` towards E; when nothing blocks (the first hit is nothing, or E itself) `ExamineSoundSource(position)`.
6. **E has a brain B:** ignore unless the world info allows `P` to be B's enemy (`CanBecomeMyEnemy(P, B's puppet, flag)`, section 6; the flag argument was not recovered here). With sensitivity **7**: `AssignFoe(B, FH_HEARD, -1)` directly. Otherwise a line-of-sight request from `P` to B's puppet: when it says **visible**, `AssignFoe(B, FH_SEEN, -1)`; when not, the ray test of 5 and `ExamineSoundSource` if unblocked.

**5.3 `ExamineSoundSource(position)`** (brain): ignored when the puppet's `en_fAllowedMovingRange` is **0 or more** (a puppet that is tied to a place does not leave it); otherwise the brain calls `OrderPuppetToGoToGoalPoint(position, pass through = false, rotate on point = true, rotation = zero)` (a go-to-point behaviour, `brain-movement.md` 5.2) and enters **`BS_EXAMINE_THREAT_SOUND`** (10). The state ends when the goal point is reached (`OnGoalPointReached`: stop the puppet, `ReconsiderBehavior`) or when a foe is assigned (`OnFoeAssigned`: `ReconsiderBehavior`). **Correction:** `brain-classes.md` 3.3 said the brain "tells the puppet to look toward the position"; it **walks** there. While in the state, `GetFoeSeekingFOVDeg` is 360.

**5.4 The "scream".** In the tracking state `TrackCurrentFoe` sends the puppet's own threat sound **once per foe**: when `en_bScreamed` is false and the time since the foe was assigned exceeds **1.0 s plus the puppet's `en_fRnd`** (a per-instance random number, **medium-low**: the member's range is not extracted), it sets `en_bScreamed` and emanates a threat sound from the puppet: **every brain without a foe within the radius gets a chance to notice the fighting puppet** (the group wakes up). `AssignFoeNow` clears `en_bScreamed`.

**5.5 Tracking, near switch** (`TrackCurrentFoe`, other parts): the **auto-lose period** (`en_fAutoLoseFoePeriod`, -1 = never) loses the foe when the time since assignment exceeds it; a line-of-sight request per period records `en_tmFoeLastSeenTime` and position; when the **closest possible foe** (world info) is not the current foe and is nearer than the third range of the `.cb` (`cb_fThreatRange`, **medium-low**, by order) the manager assigns it as **`FH_NEAR`** (5); the 60 s rule of `behavior-entity-classes.md` 7.3 applies when the foe is not seen.

### 6. What the world info test checks (**medium**)

`CanBecomeMyEnemy(a, b, flag)`: **false** when either puppet is missing, a is b, **both have the same non-empty `en_idPuppet`** (the same character id), or b is a **player puppet whose flag bit 3 is set** (a flag word of the player puppet that was not named: probably the "not a target" cheat, **low**); otherwise, with `flag` false (the candidate scans of section 4) the answer is `CanBeEnemyToMyAlignment(a's alignment, b's alignment)`. With `flag` true (callers not identified) a **hostility graph** branch is tried first: when a global switch is set, both puppets have parameter files and either puppet has `en_bCanAttackOwnAligment`, a second global switch and a per-character-id test (a helper that was not read) can make the answer yes before the alignment rule; it was not read further (T01-156). `DoIHavePossibleFoe` asks the same question over the whole list (any candidate), `GetFoeCandidates` returns every candidate that passes, optionally only players.

### 7. The alignment rule (`CanBeEnemyToMyAlignment(a, b)`; **medium**, a 4 x 4 table, script section 3)

`EBrainAligment`: 0 good, 1 evil, 2 neutral, 3 semi-neutral. Rule, in order: either is **neutral** (2): no; `a` is **semi-neutral**: yes only when `b` is **evil**; `b` is semi-neutral (and `a` is not): no; otherwise yes when `a != b`. Table (row asks, column is the other):

| a \ b | good | evil | neutral | semi-neutral |
|---|---|---|---|---|
| good | no | yes | no | no |
| evil | yes | no | no | no |
| neutral | no | no | no | no |
| semi-neutral | no | yes | no | no |

It is **not symmetric**: a semi-neutral puppet may take an evil one as a foe, an evil puppet never takes a semi-neutral one. Data: 72 of the 82 behaviors are evil, 5 good, 2 neutral, 3 semi-neutral (`ai.md`).

### 8. Data (script)

All numbers from `python -I tools/re/brain_senses_verify.py` (2,328 placed puppets of the 59 worlds, 82 `.cb` files):

- **Ranges of the `.cb` files:** active range 250 (67 files), 2,500 (13), 400 (1), 25,000 (1); sense range 10 (66), 50 (8), 20 (2), 30 (1), 5,000 (2), 0 (3); threat range 10 (64), 50 (8), 25 (3), 20 (2), 5,000 (2), 0 (3). The commonest triple (active, sense, threat) is (250, 10, 10) in 58 files; the three files with sense range 0 are the amoeba files (no states).
- **Alignment of the `.cb` files:** evil 72, good 5, semi-neutral 3, neutral 2. Placed puppets: evil 2,012, neutral 151, semi-neutral 117, good 48.
- **Default threat sensitivity of the placed puppets:** `TS_STANDARD` 1,159, `TS_LONGSIGHTED` 562, `TS_SEETHROUGHWALLS` 246, `TS_DEADHORSE` 150, `TS_DAMAGEANDSENSE` 121, `TS_SHORTSIGHTED` 64, `TS_DAMAGEONLY` 24, `TS_HEARTHROUGHWALLS` 2. So 295 puppets (13 %) never look for a foe by sight (sensitivity 1 to 3) and **246 see through walls**; 562 are unlimited-sight.
- **Overrides:** field of view stored -1 (use the global) in 2,297; 31 override it: 100 (12), 90 (11), 360 (7), 45 (1). Instance sense range 0 or more in 506 puppets (20 in 122, 15 in 60, 10 in 47, 40 in 40, 80 in 30 ...), -1 in 1,822. Short-sight distance: of the 64 puppets of sensitivity 4, 59 have a negative value (global 40) and 5 their own. `en_bDisableThreatSoundCheck` set in 791 puppets (they ignore all sounds); `en_fAllowedMovingRange` 0 or more in 222 (0 in 109, 0.1 in 62, 5 in 11 ...; those puppets never walk to a sound).
- **Sound reaction summary:** of the 2,328 puppets, 791 ignore sounds by the switch, 295 by sensitivity 1 to 3 (the two groups overlap, not counted here) and 222 do not walk to a non-brain sound.

## Invariants

1. The three global senses numbers in the data are 100 (standard see distance), 40 (short-sighted) and 360 (field of view), and the three threat distances 100 (`rsc-ctl.md`).
2. `CanBeEnemyToMyAlignment` returns false whenever either alignment is neutral (script table, **high** as a statement about the table; the rule itself **medium**).
3. A brain hears threat sounds only while it is in the "brains without foe" list, i.e. while its foe manager looks (2 functions read, **medium**).
4. With sensitivity 1 or 2 a foe is never assigned by the foe manager's own search (1 function, **medium**); foes then come only from damage, touch, hunt, force (`behavior-entity-classes.md`).

## Open questions

1. **The line-of-sight service** (the request object, its range, its answer timing, the ray test of 5.2) and `CTurretPuppetEntity::ShouldExamineThreatSound` (T01-244).
2. **`CalculateSleepTime`** and the periods of the search and tracking loops, `en_lafLastAssignedFoe`, the hostility graph branch, the `FH_NEAR` interval (T01-156).
3. **Which `.cb` member is which range** (declared order only), the range of `en_fRnd` and whether `ggp_fProjectileThreatDistance` is used (T01-244).
4. **Run-time check** of the search order and the sound reactions (T01-130, T01-038).

## Test vectors

- **See distance:** sensitivity 5: 100. Sensitivity 4, short-sight distance 25: 25; short-sight distance -15 (the data's lowest): 40. Sensitivity 6: 3.0e38.
- **Field of view:** FOV 360: every candidate passes. FOV 90, candidate at 50 degrees from the facing: `cos 50 = 0.643 < cos 45 = 0.707`: **rejected**; at 40 degrees: accepted.
- **Search:** sensitivity 5, active range 250, sense range 10, see distance 100, FOV 360: candidate at 300: ignored; at 8: `FH_SENSED`; at 60 with a clear line: `FH_SEEN` after the line-of-sight answer; at 150: ignored by the see distance (it is nearer than the active range but beyond 100).
- **Sensitivity 8:** a candidate at 200 (inside the active range 250) is assigned `FH_SEEN` at once.
- **Sound:** sensitivity 5, weapon fire of a non-brain emitter 80 away with a clear ray: the puppet walks to the position (`BS_EXAMINE_THREAT_SOUND`); the same with `en_fAllowedMovingRange` 20: ignored; with a marker mover running: ignored; sensitivity 4 with see distance 40 and the emitter 80 away: ignored; sensitivity 7 and a brain emitter: `FH_HEARD` foe without any test of sight.
- **Alignment:** good vs evil yes; evil vs good yes; good vs good no; semi-neutral vs evil yes; evil vs semi-neutral no; neutral vs anything no.
- `tools/re/brain_senses_verify.py` prints: ranges (active 250 in 67 files, 400, 2,500 in 13, 25,000; sense 10 in 66, 0 in 3, 20, 30, 50 in 8, 5,000 in 2; threat 10 in 64, 0 in 3, 20, 25, 50 in 8, 5,000 in 2), alignment 5 / 72 / 2 / 3, the world counts of section 8 and the 4 x 4 table.
