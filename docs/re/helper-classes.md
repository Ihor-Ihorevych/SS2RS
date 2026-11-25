# Helper, event and global parameter classes of `Sam2Game.dll`: overview (T01-113, part 12 of T01-026)

- **Roadmap task:** T01-113 (the twelfth and last part of the class inventory T01-026; the other parts are T01-102 to T01-112). Pages of this part: [`helper-classes-iterators.md`](helper-classes-iterators.md) (the 17 entity iterators, `CEntityArray`, `CEntityHandle`), [`helper-classes-events.md`](helper-classes-events.md) (18 animation events, 4 event handlers, the static model's event data and two terminate records), [`helper-classes-params.md`](helper-classes-params.md) (the global parameter classes, `CLauncher`, `CTextToSay`, `CToolEntityCreator`, `CFeelerSetup`). The reconciliation of all 932 exported classes with the twelve parts is [`class-inventory.md`](class-inventory.md).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/helper_classes_verify.py` reproduces from the data and the export names (the 59 classes and 471 functions, which 30 are in the data and under which name, the member lists of the 8 data parameter classes, the counts of animation events per class, the equality of the constructors' defaults with the data for the records listed on the parameter page); **medium** for what each iterator, event and handler does (each function was read once, in full, and the callers were found from the references to the class); **low** for the roles of the unnumbered members guessed from names and for three iterators nobody constructs.

Prerequisites: `conventions.md`, `classes.md` and `classes-catalog.md` (the data's type tables), `ans.md` (animation events), `rsc-ctl.md` and `rsc-ctl-schema.md` (the `.rsc` files), `player-rules.md` (the global numbers), `replication.md` (net behaviour), and the eleven sibling parts (see the table in [`class-inventory.md`](class-inventory.md)).

## Summary

`Sam2Game.dll` exports **59 classes** that are neither entities, nor menus, nor HUD elements, nor parameter classes of one family: small **helper** classes that the other families use. They export **471 functions**: **391** are the standard functions every class has (two constructors, destructor, two operators, a special function and `mdGetDataType`; plain structures have only 3 to 6 of them), **80** are named methods on 43 classes. **30 of the 59 are in the data** (their type tables name the class or its `S` twin) and **two more are the data's records under other names**; **29 exist only in the binary**.

| Group | Classes | Functions | In the data | Page |
|---|---|---|---|---|
| Entity iterators (`C...Iterator`) | 17 | 136 | 0 | [iterators](helper-classes-iterators.md) |
| Entity array and handle | 2 | 20 | 0 | [iterators](helper-classes-iterators.md) |
| Animation events (`C...AnimEvent`) | 18 | 150 | 16 | [events](helper-classes-events.md) |
| Event handlers, the static model's event data, the terminate records | 7 | 67 | 4 | [events](helper-classes-events.md) |
| Global and record parameter classes | 11 | 71 | 8 (+2 under other names) | [params](helper-classes-params.md) |
| `CLauncher`, `CTextToSay`, `CToolEntityCreator`, `CFeelerSetup` | 4 | 27 | 2 | [params](helper-classes-params.md) |
| **Total** | **59** | **471** | **30** | |

| Fact | Value | Basis |
|---|---|---|
| Classes / exported functions | 59 / 471 = 391 standard + 80 named methods on 43 classes (16 classes have none) | script, section 1 |
| Base classes | iterators `CEntityIterator`; events `CAnimEvent` (one `CSamSoundAnimEvent`); handlers `CSamAnimEventHandler`; three parameter classes `CResource`; `CSamProjectParams` `CBaseProjectParams`; the rest none | type tables and member tables, medium for the 11 iterators without a table |
| Entities among them | **none**: no class has `GetNetBehavior`; no class is placed in a world | export table |
| Member tables in the DLL | 14 for classes of this family: 13 `S` twins (6 iterators, 5 events, 2 terminate records) and `CSamTestRunParams` (the session options, section 4 of the parameter page) | tables |
| Classes under other names in the data | `CObjectManipulationParams` = `CThrowingParams`, `CLPEnvironmentParams` = `SPuppetEnvironmentParams` | defaults against data, high and medium |

**Headline findings.**

1. **None of the 59 classes is an entity or is replicated.** They are the *plumbing* of the entity families: iterators the world info and the brains run over all entities, events the animation system fires on puppets and weapons, parameter records that other parts embed. Replication rules therefore live on the parts that own them (`replication.md`). **High** (export table: none exports `GetNetBehavior`).
2. **A game-specific animation event is a one-line dispatch to a named method of its owner** (puppet or weapon), and every such method is already listed in the puppet and weapon parts; the event page supplies the missing link from the data's event class to the method (section 2 of the events page). **Medium**.
3. **The chapter system gives a player the weapons of earlier chapters through an iterator**: the brain adds each chapter's weapon set and receives every weapon pickup placed in the chapters since the last "lose weapons" chapter (iterators page, 2.2). **Medium**.
4. **Two of the record classes the data stores under different names are identified by their constructors' defaults**: the carry and throw limits (`CThrowingParams`) and the environment movement limits (`SPuppetEnvironmentParams`), the latter with a minimum and maximum instance move speed (parameter page, sections 6 and 8). **High / medium**.
5. **The session options class `CSamProjectParams` is the class of the DLL's member table `CSamTestRunParams`**, and its constructor gives the default of 32 of its 34 options (single player, Normal, 3 lives, 2 players, frag limit 20, time limit 10): parameter page section 4. **High** for the identification.
6. **The animation event flag word has 5 values in the data (0, 1, 2, 3, 4), not the three `ans.md` listed** (events page, section 3). **High** for the counts.

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were listed; the 932 exported classes were grouped by name (T01-026 split: the 59 classes are group "P11"). The 59 classes export 471 functions; names are quoted as exported. The other modules (`Engine.dll`, `Core.dll`) were not searched for further helper classes; the bases of these classes (`CEntityIterator`, `CAnimEvent`, `CAnimEventHandler`, the base events) are not exported by this DLL and are presumably defined there (`T01-207`, `T01-218`).
- **Reading.** About **120 functions** were decompiled and read in full (basis **R**): all 17 `ProcessEntity`, all 17 `Execute`, the 7 description functions, the 16 handler functions, the 10 `CEntityArray` methods, the data-structure constructors, `Initialize`, `GetNextSoundSource`, `GetInstanceMoveSpeed`, the `CEntityParams` methods and six functions of other classes that run an iterator. Callers of each class's constructor, virtual table and type record were listed, so the "user" of every iterator was found from the references, not guessed. The receiving methods of the events were identified from the virtual tables compared in the earlier parts.
- **Member tables.** The DLL's declared member tables for 14 classes of the family were read for names and parent (offsets were used only to order members and are not recorded, `conventions.md`).
- **Data.** `tools/re/helper_classes_verify.py` (about 3 minutes) reads the class tables of 8,063 files, the animation events of the 88 `.ans` files and the models that embed animations (1,313 events), `GlobalGameParameters.rsc`, `SpawningParams.rsc` and the `.ep` files for the two records, and prints sections 1 to 4 that the pages quote. It contains as constants the exported names with their function counts and the defaults that the constructors set (numbers read once from the binary).
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run; the engine side of iteration and of event execution (`Engine.dll`); the consumers of most global parameter members; the 50 member tables whose declared member names are not all spelled on some page ([`class-inventory.md`](class-inventory.md), section 6).
- **Clean room.** This part was specified by the author named above from the binary and the data; the author does not implement these topics (`CLEANROOM.md`).

## Layout / rules

### 1. Basis marks used on the pages

| Mark | Means |
|---|---|
| **R** (read) | the function was decompiled and read; the rule is medium |
| **P** (profiled) | the callers, strings and callees were listed; medium to low |
| **N** (name) | from the name only; low |
| **data** | counted or compared by the script; high |

Confidence words follow `conventions.md` section 3. Names ending in `(ours)` are invented (none on these pages); every other identifier is the binary's or the data's own, quoted as exported or declared. "Slot" is a position in a virtual function table counted from 0.

### 2. The 59 classes

| Class | Group | Parent | In the data as | Functions | Page |
|---|---|---|---|---|---|
| `CAddAllWeaponsIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CAddPastChaptersWeaponsIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CCollectChaptersForSortingIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CCollectEnvironmentDamagersIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CCollectMissionObjectivesIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CCollectNavigationalBeaconsIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CCollectSecretsIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CContributeToGameTotalStatsIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CCurrentStateInfoIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CDisableMacrosIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CExecuteMacrosIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CFindFirstEntityOfTypeIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CFindSpawnMarkerIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `COnBootIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `COnShutDownIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CPuppetsHealthDebugIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CStateCallsInfoIterator` | iterator | `CEntityIterator` | - | 8 | [iterators](helper-classes-iterators.md) |
| `CAnimEventStep` | animation event | `CAnimEvent` | `SAnimEventStep` | 9 | [events](helper-classes-events.md) |
| `CAttachToolAnimEvent` | animation event | `CAnimEvent` | `CAttachToolAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CDetachToolAnimEvent` | animation event | `CAnimEvent` | `CDetachToolAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CDrawToolAnimEvent` | animation event | `CAnimEvent` | `CDrawToolAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CHideAttachmentAnimEvent` | animation event | `CAnimEvent` | `CHideAttachmentAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CHidePlayerWeaponAttachmentAnimEvent` | animation event | `CAnimEvent` | `CHidePlayerWeaponAttachmentAnimEvent` | 9 | [events](helper-classes-events.md) |
| `CMeleeDamageAnimEvent` | animation event | `CAnimEvent` | `SMeleeDamageAnimEvent` | 9 | [events](helper-classes-events.md) |
| `CPuppetAppearAnimEvent` | animation event | `CAnimEvent` | - | 8 | [events](helper-classes-events.md) |
| `CPuppetDisappearAnimEvent` | animation event | `CAnimEvent` | `CPuppetDisappearAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CPuppetExplodeAnimEvent` | animation event | `CAnimEvent` | - | 8 | [events](helper-classes-events.md) |
| `CPuppetFireAnimEvent` | animation event | `CAnimEvent` | `SPuppetFireAnimEvent` | 9 | [events](helper-classes-events.md) |
| `CPuppetReleaseSpawneeAnimEvent` | animation event | `CAnimEvent` | `SPuppetReleaseSpawneeAnimEvent` | 9 | [events](helper-classes-events.md) |
| `CPuppetSoundAnimEvent` | animation event | `CSamSoundAnimEvent` | `SPuppetSoundAnimEvent` | 7 | [events](helper-classes-events.md) |
| `CRagdollAnimEvent` | animation event | `CAnimEvent` | `SRagdollAnimEvent` | 9 | [events](helper-classes-events.md) |
| `CShowAttachmentAnimEvent` | animation event | `CAnimEvent` | `CShowAttachmentAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CShowPlayerWeaponAttachmentAnimEvent` | animation event | `CAnimEvent` | `CShowPlayerWeaponAttachmentAnimEvent` | 9 | [events](helper-classes-events.md) |
| `CStoreToolAnimEvent` | animation event | `CAnimEvent` | `CStoreToolAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CThrowDeathToolsAnimEvent` | animation event | `CAnimEvent` | `CThrowDeathToolsAnimEvent` | 8 | [events](helper-classes-events.md) |
| `CDoorAnimEventHandler` | handler or record | `CSamAnimEventHandler` | `CDoorAnimEventHandler` | 9 | [events](helper-classes-events.md) |
| `CPuppetAnimEventHandler` | handler or record | `CSamAnimEventHandler` | `SPuppetAnimEventHandler` | 12 | [events](helper-classes-events.md) |
| `CStaticModelAnimEventHandler` | handler or record | `CSamAnimEventHandler` | `SStaticModelAnimEventHandler` | 12 | [events](helper-classes-events.md) |
| `CWeaponAnimEventHandler` | handler or record | `CSamAnimEventHandler` | - | 11 | [events](helper-classes-events.md) |
| `CStaticModelAnimEventsData` | handler or record | - | `SStaticModelAnimEventsData` | 9 | [events](helper-classes-events.md) |
| `CEffectAnimEventToTerminate` | handler or record | `CScriptObject` | - | 7 | [events](helper-classes-events.md) |
| `CSoundAnimEventToTerminate` | handler or record | `CScriptObject` | - | 7 | [events](helper-classes-events.md) |
| `CBulletParameters` | parameter | - | `CBulletParameters` | 4 | [params](helper-classes-params.md) |
| `CEntityParams` | parameter | `CResource` | `CEntityParams` | 13 | [params](helper-classes-params.md) |
| `CGlobalGameParams` | parameter | `CResource` | `CGlobalGameParams` | 7 | [params](helper-classes-params.md) |
| `CGlobalSpawningParams` | parameter | `CResource` | `CGlobalSpawningParams` | 7 | [params](helper-classes-params.md) |
| `CLPEnvironmentParams` | parameter | - | `SPuppetEnvironmentParams` (other name) | 7 | [params](helper-classes-params.md) |
| `CMaterialSoundRanges` | parameter | - | `CMaterialSoundRanges` | 4 | [params](helper-classes-params.md) |
| `CObjectManipulation` | parameter | - | `CObjectManipulation` | 6 | [params](helper-classes-params.md) |
| `CObjectManipulationParams` | parameter | - | `CThrowingParams` (other name) | 6 | [params](helper-classes-params.md) |
| `COperatingParams` | parameter | - | `SOperatingParams` | 6 | [params](helper-classes-params.md) |
| `CSamNetworkParams` | parameter | - | `CSamNetworkParams` | 4 | [params](helper-classes-params.md) |
| `CSamProjectParams` | parameter | `CBaseProjectParams` | - | 7 | [params](helper-classes-params.md) |
| `CEntityArray` | array or handle | - | - | 16 | [iterators](helper-classes-iterators.md) |
| `CEntityHandle` | array or handle | - | - | 4 | [iterators](helper-classes-iterators.md) |
| `CLauncher` | other | - | `SLauncher` | 11 | [params](helper-classes-params.md) |
| `CTextToSay` | other | - | - | 6 | [params](helper-classes-params.md) |
| `CToolEntityCreator` | other | - | `CToolEntityCreator` | 7 | [params](helper-classes-params.md) |
| `CFeelerSetup` | other | - | - | 3 | [params](helper-classes-params.md) |

### 3. Net behaviour and hooks

No class of the family is an entity, so no class overrides entity hooks (`OnCreate`, `OnStep` and so on) and none has `GetNetBehavior` or replicated members. The one place where a member of these classes is replicated is **inside another class's member**: `CSamProjectParams` is the game info entity's `en_sgpGameParams`, of which 28 members are flagged as replicated (`replication.md`); the network error block `CSamNetworkParams` sits in the global parameters, which are loaded identically by every machine. Animation events run on **whichever machine evaluates the animation**; whether a client or only the host runs the game-specific events is open (`T01-207`).

### 4. The matching `.ep` / `.rsc` schema classes

`CGlobalGameParams`, `CGlobalSpawningParams` and the embedded `CSamNetworkParams`, `CBulletParameters`, `CMaterialSoundRanges` are described member by member in `rsc-ctl-schema.md`; `SOperatingParams`, `CObjectManipulation`, `CThrowingParams` and `SPuppetEnvironmentParams` in `ep-schema.md`; the event classes in `ans.md`; the static model event data and the three handlers' saved forms in `wld-entity-schema.md`.

## Invariants

1. 59 distinct exported class names, 471 exported functions, 30 in the data (script, section 1).
2. No class of the family exports `GetNetBehavior` (59 of 59).
3. The 17 iterators and the 2 array and handle classes are in no type table (19 of 19); 16 of the 18 events, 3 of 4 handlers, `CStaticModelAnimEventsData`, 8 of 11 parameter classes, `CLauncher` and `CToolEntityCreator` are.
4. The per-group function counts add up to 471: 136 + 20 + 150 + 67 + 71 + 27.
5. Every class of this page is named on exactly one of the three companion pages (checked in [`class-inventory.md`](class-inventory.md)).

## Open questions

Each is a follow-up line in the roadmap (`T01-207` to `T01-217`, `T01-219`):

1. **T01-207** the engine side of animation events: arguments of `Execute`, the flag bits, the sound source type names, the seven base events, which machine evaluates events.
2. **T01-208** who runs the three iterators that no exported function constructs, and the debug dump configuration.
3. **T01-209** the boot iterator's exact condition and the order of a chapter restore.
4. **T01-210** which start marker single player and co-op use; the last test of the spawn marker iterator.
5. **T01-211** the engine's iteration service.
6. **T01-212** consumers of the 80 global parameter members and the meaning of the second carry record.
7. **T01-213** the six spawning graphs.
8. **T01-214** the environment record's roles, the callers of `GetInstanceMoveSpeed`, `CFeelerSetup`.
9. **T01-215** which members of which classes are entity arrays.
10. **T01-216** the member names of the tables that the pages do not spell out (inventory section 6).
11. **T01-217** the meaning of the session options' -1 defaults and the readers of each option.
12. **T01-219** the static model event data's initialisation.

## Test vectors

See the three companion pages (a spawn marker choice, an array removal, the instance move speed, a freshly constructed session option set, the sound source ring).
