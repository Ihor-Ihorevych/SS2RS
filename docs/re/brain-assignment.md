# Which brain each puppet gets (T01-125)

- **Roadmap task:** T01-125 (follow-up of [`brain-classes.md`](brain-classes.md) open question 1; companion pages: [`puppet-classes.md`](puppet-classes.md), [`puppet-entity-classes.md`](puppet-entity-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** only for the counts that `tools/re/brain_assignment_verify.py` reproduces from the data (which puppet entity classes the worlds place, how many of them link a character behavior, which parameter files they use, how the behavior files pair with them, how many parameter files declare seats); **medium** for every rule read from the binary (the creation rule, its caller and its conditions, the brain's behaviour with and without a `.cb`, the registration of rides), because each rests on one or two readings of the same few functions and **nothing was run**; **low** for what is marked "(name)" or listed under "Open questions".

Prerequisites: [`brain-classes.md`](brain-classes.md) (the brain, its states, the creation rule this page re-reads), [`puppet-classes.md`](puppet-classes.md) (Character entities, obsolete kinds, dummy foe, generic legged), [`puppet-entity-classes.md`](puppet-entity-classes.md), `ai.md` (the `.cb` files), [`replication.md`](replication.md), [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md).

## Summary

`brain-classes.md` section 2.3 gave the rule by which a puppet gets its brain from one reading of one function. This page re-reads it a second time, adds **who calls it and under which conditions** (it was not known), settles **what happens without a `.cb`**, **how a puppet gets its `.cb`**, **why the per-kind brains are dead**, **what the two live shells are for**, and corrects two statements of `brain-classes.md`.

1. **Every puppet that a shipped world places gets the plain `CBrainEntity`** (2,328 of 2,328 placed puppets; **medium**). The five entity classes that the worlds place are all "Character" classes whose names contain no `PuppetEntity`, so the name pairing of `CreateBrain` cannot find a per-kind brain for any of them; with or without a `.cb` the result is the base brain. The per-kind classes (`CKleerBrainEntity` ... `CWheeledVehicleBrainEntity`) are only reachable for a puppet class whose name contains `PuppetEntity` **and** that has no `.cb`; no world places such a puppet (section 3).
2. **`CreateBrain` is called from exactly one place, `CPuppetEntity::OnBoot`, and only on a host** (single player, listen host, dedicated server) **and only for a puppet whose spawner link is empty and that is not a multi-spawner template** (section 1). A client never creates a brain for a puppet it received: on a client every puppet except the player's own has **no brain at all** (**medium**). The placed template puppets of spawners (a spawner link is set) are never booted, so they never get one; the copies that a spawner makes have the link broken and get one when they boot.
3. **A puppet without a `.cb` gets a brain that does nothing** (section 4): no foe manager, state `BS_DEFAULT`, no think loop, no mover. 52 of the 2,328 placed puppets are of that kind (7 aircraft, 17 hovercraft, 6 legged, 22 turrets; 40 of the 52 have seats and are rides, section 6).
4. **How a puppet gets its `.cb`**: not from its character name. The placed record stores a **pointer to the `.cb` file** (`ep_pcbCharacterBehavior`, 4,696 set and 87 null in the 4,783 records of the 59 worlds); only when that pointer is empty does the puppet fall back to an older **identifier** member (`ep_idBehavior`), which is looked up in a table of the **file names of every `*.cb` file found under `Behaviors/`**. The identifier is empty in every stored record, so the fallback is dead in the shipped data (section 5).
5. **Why the four "live" kinds have obsolete brains**: they do not. The name pairing is by the class name of the *puppet entity*; the data's aircraft, hovercraft, turret and rolling-ball puppets are `...CharacterEntity` classes, which never pair. The brain classes of those kinds were left over from an older design in which a placed `CAircraftPuppetEntity` had its own brain; the 16 brains and 11 puppets that carry the shared obsolete-class error are the classes that were retired with it (section 7).
6. **The two live shells** are the brains of the **dummy foe** puppet (`CDummyFoePuppetEntity`) and of the **generic legged** puppet (`CGenericLeggedPuppetEntity`): each answers "no foe manager" so these two puppets never run the `.cb` loop. Neither puppet is in any shipped world (section 8).
7. **Two corrections to `brain-classes.md`** (section 9): `OnCreatedByPuppet` registers the **puppet as a ride** with the world info entity when the puppet has seats (it does not touch the list of brains without a foe), and the member copied at the end of `CreateBrain` is the **default threat sensitivity into the current one**.

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs (no other build compared). The functions read, in full, with their callees: `CPuppetEntity::CreateBrain`, `OnBoot`, `OnCreate` (the part that stores the behavior and the parameters), `ConnectToBrain`, `DeleteBrain`, `ChangeToBrain`, `ChangeToDefaultBrain`, `IsRidable`, `GetCountOfSeats`, `GetPuppetParameters`; `CBrainEntity::OnCreatedByPuppet`, `OnDelete`, `HasFoeManager`, `ReconsiderBehavior`, `ReconsiderBehavior2`, `OnBoot`; `CWorldInfoEntity::RegisterRide`, `UnregisterRide`, `CharacterBorn`; `CDummyFoePuppetEntity::OnCreate`; the helper that creates an object from a class name; the helper that builds the table of behavior files. **Two readings** where it matters: the function bodies and the virtual function tables (13 classes: the puppet base, the player puppet, the legged, legged-character, aircraft-character, dummy foe, generic legged, rolling ball, turret, aircraft, hovercraft and wheeled-vehicle puppets, and the brain) compared slot by slot.
- **Callers.** Every function of the DLL was scanned for an indirect call through the position that `CreateBrain` has in the puppet's virtual table. Nine functions call through that position; only one calls it on a puppet (`OnBoot`). The other eight use the same position of another class's table: the two spawnee-release functions of the puppet (read: they call it on the spawner), `PrepareForCGS` (read: it calls the brain's `ForceMovingType`), `CPossessRideEntity::SetState` (read: a call on the entity itself), and four more by name only (`CBAForceMovingType::Execute`, `CBAVTWarMeasure::IsValid`, `CWorldInfoEntity::RenderHostilityGraph`, `CGBARunOverFoeBehaviorEntity::StartApproaching`; **medium-low**: not read, the names make a puppet call implausible). A scan finds only direct virtual calls; a call by a saved function pointer or from another module (`Sam2GameGui.dll`, `Sam2.exe`) was not looked for.
- **Data.** `python -I tools/re/brain_assignment_verify.py "<install>"` (about 20 minutes: it decodes the 59 worlds, the 82 `.cb` and the puppet parameter files; `LIMIT` reads only the smallest worlds) prints the sections 1 to 5 quoted below.
- **Not checked.** Nothing was run: there is no instrumentation of the original, so "the running original" of the roadmap line (which brain class a live puppet has, the order of the calls on a client) is **not observed** and stays **T01-130**. The path by which a client receives a puppet and whether it boots it (the engine's boot pass) was not read; what is established is that the boot hook of the puppet does not create a brain on a client.

## Layout / rules

### 1. When `CreateBrain` runs (**medium**)

`CreateBrain` is a virtual method of `CPuppetEntity`. No puppet class overrides it (13 virtual tables compared; the player puppet overrides only `DeleteBrain`).

Its **only caller** is `CPuppetEntity::OnBoot`. The first block of `OnBoot` (placement, the first `PS_` state set-up, mechanism, default riders and tools, attachments, the "alive" sound) runs **only when both** hold:

1. the puppet's **spawner link** (`en_henSpawner`) is empty, and
2. `en_bMultiSpawnerTemplate` is false.

Inside that block the order is: tell the world info entity the character is born (`CharacterBorn`: adds the puppet to the world info's list of existing puppets, which is the candidate list of the foe search, and, for an evil puppet with health below 8,000, to the list of living enemies), set the placement, `en_bAlive` = true, then **on a host only** (`netIsHost`): `CreateBrain`, then (every machine) `AssureThatOnStepIsEnabled`, then on a host only the default riders (`SetupDefaultRiders`) and the clearing of the default-rider list, then the tools and attachments. The "alive effects" are spawned after the block for a puppet without spawner link.

Consequences (**medium**):

- **A client does not create brains.** `OnBoot` can run on a client (the boot pass is engine code), but the creation is behind `netIsHost`. The puppet's brain link (`en_henBrain`) is not replicated (flags: none; `replication.md`), the brain entity is `NEB_NONE`, so a client has no brain for any puppet except the **player's own**, which receives its `CPlayerBrainEntity` through `ChangeToBrain` (a player brain connecting to its puppet; `players-session-classes.md`).
- **A placed puppet with a spawner link is a dormant template** (the link `en_henSpawner` is set in **2,195 of the 2,328** placed puppets and empty in 133: 116 with a `.cb`, 17 without; script section 4; the second link, `en_henSpawnerWhoSpawnedMe`, is empty in all 2,328). It is not booted as a live puppet, so it gets no brain. The spawner makes live puppets by **copying the template's properties together with their links**, calling three virtual methods of the copy before the entity is created (by their positions in the property class's table: a spawner-link breaker whose name is not exported, `BreakMultiSpawnerLinks` and `MarkAsSpawnee`: **medium-low**) and creating the entity from it: the copy has an empty spawner link, so it takes the `OnBoot` path above. (`SpawnOneSpawnee`, read in outline.)
- **Player and ride puppets.** `CreateBrain` returns at once when the puppet **already has a brain** (the first test), so a puppet that a player brain made and connected is left alone.

### 2. The rule itself (**medium**, second reading; unchanged in outcome from `brain-classes.md` 2.3, sharpened in detail)

`CreateBrain`, in order:

1. If the puppet already has a brain (`en_henBrain` valid): **stop**.
2. If the puppet **has no character behavior** (`en_pcbCharacterBehavior` null): take the **class name** of the puppet entity (the class-name identifier of its type), replace the text `PuppetEntity` by `BrainProperties` (the engine's `strReplaceSubstrB`; whether it replaces the first or every occurrence does not matter: the names have one), and ask the engine to **create an object of the class of that name**. The helper does so only when a class of that name exists and derives from the engine's entity-properties class `CEntityProperties` (the imported type that both the helper and `CreateBrain` test against); otherwise it returns nothing. A found object is accepted only when it derives from `CBrainProperties`; it then gets the puppet's **original placement** (`en_qvOriginalPlacement`, seven floats) written into its placement member, **is spawned**, and the entity it makes is accepted as the brain only when it derives from `CBrainEntity` (stored in `en_henBrain`). The temporary properties object is deleted afterwards. **With a behavior this step is skipped.**
3. If there is **still no brain**, create a plain `CBrainProperties` object in memory, give it the same placement, spawn it, and if the entity derives from `CBrainEntity` store it in **both** `en_henBrain` and `en_henDefaultBrain`; the temporary object is deleted. If step 2 made a brain, copy `en_henBrain` into `en_henDefaultBrain` instead.
4. Copy **`en_tsDefaultThreatSensitivity` into `en_tsCurrentThreatSensitivity`** (the member `brain-classes.md` called "an initial sight or perception mode, name unknown": the `TS_` enumeration of `puppet-classes.md` 3.3, values 1 to 8).
5. `ConnectToBrain(brain)`: stores the brain in `en_henBrain` and writes the puppet into the brain's puppet link.
6. Call the brain's `OnCreatedByPuppet` (sections 4 and 6).

The outcome is the same as in `brain-classes.md` 2.3. Two points are new: the object that the name produces is a **properties object that is spawned**, so a class name that is an entity class never produces a brain (section 3); and the final steps (the threat sensitivity copy, `ConnectToBrain`, `OnCreatedByPuppet`) run for every brain.

### 3. The result for every placed puppet class (**medium** for the rule, **high** for the class names and counts)

The shipped worlds place **five** puppet entity classes, all of the `...CharacterEntity` kind (`puppet-classes.md` 3.1; script, section 1). `

| Entity class (data name) | Placed | With a `.cb` | Without | Parameter class |
|---|---|---|---|---|
| `SLeggedCharacterEntity` | 1,825 | 1,819 | 6 | `SLeggedPuppetParams` |
| `SAircraftCharacterEntity` | 254 | 247 | 7 | `SAircraftPuppetParams` |
| `STurretCharacterEntity` | 164 | 142 | 22 | `STurretPuppetParams` |
| `SRollingBallCharacterEntity` | 44 | 44 | 0 | `SRollingBallPuppetParams` |
| `SHovercraftCharacterEntity` | 41 | 24 | 17 | `SHovercraftPuppetParams` |
| **Total** | **2,328** | **2,276** | **52** | |
`

None of the five names contains the text `PuppetEntity` (0 of 2,328). Step 3 of the rule therefore asks for a class whose name is the entity's own (`CLeggedCharacterEntity` and so on): that name is an **entity** class, which does not derive from `CEntityProperties`, so the helper returns nothing (the check is on the type the name resolves to, **medium**) and step 4 builds the plain base brain. So **for all five classes, with or without a `.cb`, the brain is `CBrainEntity` itself**. The answers to the roadmap line's cases:

| Case | Brain | Basis |
|---|---|---|
| Placed enemy (legged, with `.cb`) | `CBrainEntity`, with a foe manager | rule, script (1,819 of 1,825 legged have a `.cb`) |
| Placed enemy or animal without a `.cb` (6 legged: Chicken 2, Santa, Simba doll, male, shaman) | `CBrainEntity`, **without** a foe manager (section 4) | rule |
| Rideable vehicle or turret with a `.cb` (aircraft 247 of 254, hovercraft 24 of 41, turrets 142 of 164) | `CBrainEntity` with a foe manager, **and** the puppet is registered as a ride (section 6) | rule |
| Rideable vehicle or turret without a `.cb` (40 placed, section 6) | `CBrainEntity` without a foe manager, registered as a ride | rule |
| Rolling ball (44 placed, all with a `.cb`: `RollingBall.ep` pairs with `rollingball.cb` or `vehicleattack_simple.cb`) | `CBrainEntity` with a foe manager, registered as a ride | rule, script |
| The player's puppet (`CPlayerPuppetEntity`) | `CPlayerBrainEntity`, connected by the player brain (`ChangeToBrain`); `CreateBrain` leaves it | `players-session-classes.md`; the first test of the rule |

### 4. What the brain does without a character behavior (**medium**)

- `HasFoeManager` is true exactly when the puppet's `en_pcbCharacterBehavior` is non-null. Without a `.cb`: `OnCreatedByPuppet` does **not** spawn a foe manager (`en_henFoeManager` stays empty). `HasFoe` is false, `AssignFoe` and `LoseFoe` do nothing, no foe is ever assigned.
- The brain's own boot hook calls `ReconsiderBehavior`, which clears the behavior entity and, a moment later, runs `ReconsiderBehavior2`. That function **enters `BS_DEFAULT` and does nothing else when the puppet is missing or has no `.cb`**; with a `.cb` it spawns the think-point oscillator and enters `BS_DEFAULT_CHARACTER_MOVING`. So **a behaviorless brain has no think loop, no mover and no `.cb` state**: the puppet is moved only by scripts and macros (patrol, goal points, rides), by physics and by being damaged (the damage hooks find no behavior and raise no moment).
- `StartCharacterAction` with no behavior records the text "Unknown starting character behavior is set." as the reject reason and starts nothing (`brain-classes.md` 3.2).
- `OnCreatedByPuppet` still stores the puppet as default puppet, registers a ride (section 6) and enters the state the puppet stores in `en_idDefaultState` (`SetNewState`), which raises `EndState`/`BeginState` against a behavior that does not exist: nothing happens.

### 5. How a puppet gets its behavior (**medium** for the code, **high** for the data)

`CPuppetEntity::OnCreate` copies values from the placed record into the live puppet. For the behavior it does this, in order:

1. If the record's `ep_pcbCharacterBehavior` pointer is **empty**, look at the record's older identifier `ep_idBehavior` (a name): when it is not the empty string, look it up in the table of behavior files and load that resource as the `.cb`, store it into the record's pointer and **clear the identifier**. (The member is named `ep_idBehaviorObsolete` in the binary: the code that uses it is a compatibility path.)
2. Copy the record's pointer into the puppet's `en_pcbCharacterBehavior`.

The **table of behavior files** is built once, when the game starts: every file matching `*.cb` under the `Behaviors/` folder of the game's database folder is found (recursively), and each entry is the file's path and an identifier made from **the file name** by the engine's file-name function (`strFileName`; whether the extension is dropped, and the case rule, are not established, T01-241; the data's own `cb_idName` equals the file stem in 74 of 82 files, which suggests the stem). A lookup is by identifier equality. **It is not the `cb_idName` stored inside the file** that is the key: `ai.md` found `cb_idName` equal to the file stem in 74 of 82 files; the table uses the file name.

The parameters follow the same pattern with `ep_idPuppet` (also obsolete) and `GetPuppetParameters` (`puppet-classes.md` 3.2).

**Data** (script sections 1 to 4, all 59 worlds):

- `ep_pcbCharacterBehavior`: **4,696 set, 87 null** in 4,783 records; `ep_idBehavior` and `ep_idPuppet`: **empty in every record** (`wld-entity-schema.md`). In the 2,328 placed entities: `en_pcbCharacterBehavior` set in 2,276 and null in 52 (same as `wld-entity-schema.md`). The 52 without a pointer are placed in the worlds with the parameter files `HoverSaucer` 8, `Turret_Plasma` 6, `HoverFighter` 5, `Kwongo_MachineGun` 5, `Turret_MachineGun` 4, `PlayerHelicopter` 4, `KwongoCatapult` 4, and 16 more in 11 files with 1 or 2 each (script, section 2).
- **The pairing is by pointer, not by name**: a parameter file is paired with 1 behavior file in 93 cases and with several in 20 (for example `Simba_Male.ep` with `npc_civilian`, `npc_civilian_good`, `npc_civilian_static` and `npc_warrior`; `Tank.ep` with `tank` and `tank_fast`; `RollingBall.ep` with `rollingball` and `vehicleattack_simple`). The level designer chooses the `.cb` per placed puppet; the parameter file is not a key to it. Of the 82 `.cb` files **81 are used by at least one placed puppet** (`airattack.cb` is not).
- **The default `.cb` state** (`en_idDefaultState`, which `OnCreatedByPuppet` enters) is a state of the puppet's own behavior in 2,175 of the 2,276 puppets with a `.cb`; in the other 101 it is not (`ameba` has no state `Default`: 77 puppets, `ameba_semineutral` 10, `ameba` with `Lasers` 9 and `Bombs` 2, `amebaevil` 1, `rhino` with `Shooter` 1, `scifi_gunner` with `Guard` 1). What the brain does with a state id that its behavior lacks was not read (T01-126).

### 6. Rides (**medium**)

`CBrainEntity::OnCreatedByPuppet` asks the puppet whether it is **ridable** (`IsRidable`: the puppet's parameter file declares at least one seat: the count is the length of the parameter file's seat container, `SPuppetParams.4`) and, when so, tells the world info entity to **register the puppet as a ride** (`RegisterRide`, which adds the puppet to the world info's list of rides `en_eaRidesInExisting` unless it is already in it). `CBrainEntity::OnDelete` does the reverse (`UnregisterRide`). The list is read by the ride bookkeeping (`CheckRidesInExistingValidity`, the boarding logic of T01-128); it is **not** the list of brains without a foe. So **every puppet with seats that gets a brain is registered**, with or without a `.cb`.

The data (script, section 5; the seat count is the length of the parameter file's seat container):

| Entity class | Seats, with `.cb` | Seats, no `.cb` | No seats, with `.cb` | No seats, no `.cb` |
|---|---|---|---|---|
| Legged | 155 | 0 | 1,664 | 6 |
| Aircraft | 31 | 5 | 216 | 2 |
| Turret | 11 | 18 | 131 | 4 |
| Rolling ball | 6 | 0 | 38 | 0 |
| Hovercraft | 16 | 17 | 8 | 0 |
| **Total** | **219** | **40** | **2,057** | **12** |

So 259 of the 2,328 placed puppets have seats and are registered as rides (219 of them with a foe manager); 40 of the 52 behaviorless puppets are rides (the player's helicopter 4, hover saucers and fighters 17, the turrets that carry a seat 18, one simba bomber), 12 are not (6 legged: the chickens, Santa, the simba doll, one male and one shaman; 2 aircraft `Fighter02_Mental`; 4 turrets). The files with at least one seat include not only vehicles and turrets but also some **mounts that are legged characters** (`albino`, `dino_m`, `dino_s`, `harpy`, `lizard`, `rhino`, `robotfloater`, 155 placed legged puppets with seats): their brain is the normal `.cb` brain and they are rides in addition.

### 7. Why the per-kind brains are dead (**medium**)

- The pairing is by the **name of the puppet entity class**. The classes the data uses are `...CharacterEntity` (no `PuppetEntity` in the name), so they never pair; the old `...PuppetEntity` classes of the 16 kinds are the ones that pair, and those that are obsolete print the obsolete-class error from the puppet's own `OnCreate` (the same shared function as the brains': `puppet-classes.md` 3.4), which does nothing else, so such a puppet is never built.
- **Aircraft, hovercraft, turret, wheeled vehicle.** Their *puppet* classes are live because the Character classes derive from them (`CAircraftCharacterEntity` from `CAircraftPuppetEntity`, and so on); their *brain* classes are marked obsolete. Both statements are consistent: the pairing is for the base `CAircraftPuppetEntity` **without a `.cb`**, a class nobody places. No world places an `S...PuppetEntity` class of any kind (0 in 2,328), the data has only the Character classes, so the obsolete brain of these four is never reached by the shipped data. If a modded world did place a base `CAircraftPuppetEntity` without a `.cb`, its brain would be `CAircraftBrainEntity`, which prints the obsolete-class error from its `OnCreate` and is then a bare brain (the same as a behaviorless base brain).
- **The shared obsolete function** is also the `OnCreate` of 11 puppet kinds, two spawn effects and three items; it prints the message and returns. The kinds were retired together; why these four brains were retired while their puppets were kept is **not recoverable from the binary** (the behaviour of the live kinds moved into `.ep` parameter files and Character classes; **low**).

### 8. The dummy foe and generic legged shells (**medium**)

- `CDummyFoePuppetEntity` has no `.cb` (it loads the built-in parameter file `Puppets/Objects/DummyFoe.ep`, which is not in the install; its `OnCreate` sets alignment, flying, velocity and health from its properties). Its class name contains `PuppetEntity`, so `CreateBrain` pairs it with **`CDummyFoeBrainProperties`**: the brain is `CDummyFoeBrainEntity`, whose `HasFoeManager` answers false. A dummy foe is a **target**: it never looks for a foe, never acts, and the brain exists only to give the puppet a valid brain link (the player's weapons and foe managers of other puppets hit it as a target; the damage hooks need a brain).
- `CGenericLeggedPuppetEntity` (a legged puppet whose model comes from its properties) pairs with **`CGenericLeggedBrainProperties`** the same way: `CGenericLeggedBrainEntity`, no foe manager. It is a **scripted prop**: it can walk and be told what to do by macros (patrol, goal points), but runs no `.cb`.
- For both, the `.cb` test of step 2 happens first: if a designer gave one of these puppets a `.cb`, the base brain would be built instead (with a foe manager) and the shell would not be used. No shipped world has either puppet, so no shipped puppet uses the two shells; `CCharacterBrainEntity` (no override) is never selected by the rule at all, since the rule has no `CharacterBrainProperties` partner for any puppet class name.

### 9. Corrections to `brain-classes.md`

1. `OnCreatedByPuppet`: the question the brain asks the puppet is **`IsRidable`** (seats declared), and the call is the world info entity's **`RegisterRide`** with the puppet, not an addition to `en_eaBrainsWithoutFoe` (that list is filled by `BrainNeedsFoe` and emptied by `BrainDoesntNeedFoe`, which the foe manager calls: T01-129). `OnDelete` calls `UnregisterRide` the same way.
2. `CreateBrain`'s copy of "one of its own members" is **`en_tsDefaultThreatSensitivity` into `en_tsCurrentThreatSensitivity`**, the member that `GetSeeDistance` reads (`TS_SHORTSIGHTED` 4 and `TS_LONGSIGHTED` 6 are the two values it tests: T01-129).
3. `ReconsiderBehavior2` enters `BS_DEFAULT` for a behaviorless puppet (as stated) and also when the brain has **no puppet**.

## Invariants

1. 2,328 of 2,328 placed puppets are `...CharacterEntity` instances of 5 classes; 0 class names contain `PuppetEntity` (script).
2. `CreateBrain` has one caller on a puppet (`OnBoot`), behind `netIsHost`, spawner link empty and not a multi-spawner template (the 13 virtual tables and the scan of `Sam2Game.dll`, **medium**).
3. A brain made by `CreateBrain` is `CBrainEntity` for every placed puppet (**medium**).
4. 52 of the 2,328 placed puppets have no behavior link (script); 40 of the 52 have seats (script).
5. `ep_idBehavior` and `ep_idPuppet` are empty in all placed records (script, `wld-entity-schema.md`).

## Open questions

1. **Run-time confirmation** (T01-130): the class of the brain of a live Kleer, a hovercraft, a turret without `.cb`, and a rolling ball; that a client has no brain for non-player puppets; that a spawner copy boots with an empty spawner link. Nothing here was observed.
2. **The engine's boot pass on a client** (T01-240): which entities a client boots and when; whether `OnBoot` of a replicated puppet runs on the client before its members have arrived.
3. **Name replacement details** (T01-241): whether the replacement is first-match or all-match, the case rule of the behavior-file identifier, and the exact check of the helper that creates an object from a class name (a mod with a class named `...PuppetEntity` would need it).
4. **The three property methods that the spawner calls on its copy** (T01-241): confirm by name that they break the spawner links and mark the copy as spawned.
5. **Why exactly these four brains were retired** (the commit history is not available): not answerable from the binary.

## Test vectors

- **Rule by class name**: a puppet of class `CKleerPuppetEntity` with no behavior looks for `CKleerBrainProperties`; `CLeggedCharacterEntity` (no `PuppetEntity`) finds nothing and gets the plain `CBrainEntity`; any puppet with a behavior gets `CBrainEntity` without a lookup; a puppet that already has a brain is left alone.
- **Host only**: `CreateBrain` is called when `netIsHost` is true, the spawner link is empty and `en_bMultiSpawnerTemplate` is false; on a client no puppet gets a brain from `OnBoot`.
- **Behaviorless brain**: `Turret_Plasma` placed without a `.cb` (6 in the data): brain `CBrainEntity`, no foe manager, state `BS_DEFAULT`, no think loop, registered as a ride (the file declares seats).
- **Threat sensitivity**: a puppet with `en_tsDefaultThreatSensitivity` 4 gets `en_tsCurrentThreatSensitivity` 4 at brain creation.
- **Behavior pairing by pointer**: `Simba_Male.ep` is placed with four different `.cb` files (`npc_civilian`, `npc_civilian_good`, `npc_civilian_static`, `npc_warrior`).
- `tools/re/brain_assignment_verify.py` prints: the classes of section 1 (2,328 placed: legged 1,825, aircraft 254, turret 164, rolling ball 44, hovercraft 41; 0 names with `PuppetEntity`), 2,458 property records of which 2,406 link a behavior, the 52 behaviorless puppets by parameter file (section 2), 93 parameter files paired with one `.cb` and 20 with several (section 3), 81 of 82 `.cb` files referenced (`airattack` not), 8 of 82 file stems that differ from `cb_idName`, the default state check (2,175 yes, 101 no), the spawner-link split (2,160 + 35 set, 116 + 17 empty) and the seat table (section 5; 22 parameter files with seats).
