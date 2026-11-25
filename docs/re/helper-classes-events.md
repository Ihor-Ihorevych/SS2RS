# Animation event, event handler and event record classes of `Sam2Game.dll` (T01-113, part 12 of T01-026)

- **Roadmap task:** T01-113 (overview and table of all 59 classes: [`helper-classes.md`](helper-classes.md); the iterators and entity arrays: [`helper-classes-iterators.md`](helper-classes-iterators.md); the parameter classes: [`helper-classes-params.md`](helper-classes-params.md); all 932 classes: [`class-inventory.md`](class-inventory.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the export counts, for which of the 25 classes are in the data and under which name, for the counts of events per class in the 88 `.ans` files and the models, for the members of the data classes and their value distributions (`tools/re/helper_classes_verify.py`, section 2), and for the member names of the 7 classes whose member tables the binary declares (they agree with the data's type tables where both exist); **medium** for what each event does (every `Execute` was read in full and is 150 to 190 bytes; the receiving puppet and weapon methods are named from the virtual tables) and for the four handlers (each function read once); **low** for the meaning of the two numbers `Execute` receives and of the flag values 1 to 4.

Prerequisites: [`helper-classes.md`](helper-classes.md), `ans.md` (the `CAnimEventHolder` and the event list of an animation), `puppet-entity-methods.md` (the receiving puppet methods), `weapon-classes.md` (the receiving weapon methods), `mechanism-classes-movers.md` (doors), `worldcontent-models-props.md` (static models).

## Summary

An **animation event** is an object stored in an animation (`.ans` file or a model's embedded animation set) that fires at one frame and tells the entity that plays the animation to do something: play a sound or an effect (base classes of the engine, not in this DLL), fire a blast, deal melee damage, attach or store a tool, show or hide an attachment, step, become a ragdoll. `Sam2Game.dll` exports **18 event classes** of the game-specific kinds, **4 event handler classes** (the object that receives the events of one kind of owner and passes them to it), **one record class for the static model's sound and effect sources** and **two bookkeeping records** for sounds and effects that must be stopped later: **25 classes, 217 exported functions**. **20 of the 25 are in the data** (16 events, 3 handlers, 1 record) and the events are in all 1,313 event objects of the install (`ans.md`).

| Fact | Value | Basis |
|---|---|---|
| Classes / exported functions | 25 (18 events, 4 handlers, 1 record, 2 terminate records) / 217 | export table |
| Named methods | events: `Execute` (17 classes), `GetDescription` (5) or `mdGetName` (2); handlers: `ProduceSound`, `ProduceEffect`, `ProduceShakeEffect`, `TerminateSound`, `TerminateEffect` (door: the two sound ones only; weapon: no shake effect); record: `Initialize`, `GetNextSoundSource` | export table |
| Event classes in the data | 16 of 18 (`CPuppetAppearAnimEvent` and `CPuppetExplodeAnimEvent` are in no file); 10 under the `C` name, 6 as the `S` twin | script, section 1 |
| Events of these 16 classes in the data | 972 of the 1,313 events of the install (the other 341 are of 7 base and effect event classes) | script, section 2 |
| Handlers in the data | 3 of 4 (`CWeaponAnimEventHandler` is in no file): door 33 `.wld`, puppet 26, static model 9 | script |
| Base class | events: `CAnimEvent` (data); the puppet sound event: `CSamSoundAnimEvent`; handlers: `CSamAnimEventHandler` (data: members `saeh_henEntity`, `saeh_tmLastEvaluation`) | data type tables |
| Net behaviour | none (no entity, no `GetNetBehavior`): events run on whichever machine evaluates the animation | export table; the evaluation side is open (`T01-207`) |

**Headline findings.**

1. **Every game-specific event is a thin dispatch**: `Execute` checks that the handler is of the expected kind (puppet or weapon), takes the **owner entity** the handler holds, checks the owner's class once more, and calls **one named method** of the owner with the event's one member (an identifier or a period). It returns 0 in all 17 classes that define it. A wrong handler or owner prints one line to the console ("... can be started only on puppet entities.") and does nothing (section 2). **Medium**.
2. **The receiving methods are exactly the `OnAnimEvent...` methods of `CPuppetEntity` and `CBaseWeaponEntity`** that `puppet-entity-methods.md` and `weapon-classes.md` already list; this page gives the missing link from the data's event class to the method (section 2, table). **Medium** (names from the virtual tables, arguments from the calls).
3. **`CRagdollAnimEvent` and `CAnimEventStep` need a walking puppet**: they refuse owners that are not `CLeggedPuppetEntity` (section 2). **Medium**.
4. **The data's flag word is not only 0, 1 and 4**: over the 1,313 events `ae_ulFlags` is 0 (1,189), 1 (92), 2 (19), 3 (3) and 4 (10), two more values than `ans.md` lists (section 3). **High** for the counts, **low** for the meaning.
5. **A static model animates through a ring of four sound sources**: `CStaticModelAnimEventsData` holds four sound sources and hands them out in turn, so four overlapping sounds of one static model do not cut each other (section 4). **Medium**.

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were listed; the 932 exported classes were grouped by name (T01-026 split); the 25 classes of this page are the event, handler and record classes of group "P11" (the group's other 34 classes are on the neighbouring pages).
- **Reading.** All 17 `Execute` functions, all 7 `GetDescription` or `mdGetName` functions, the 16 handler functions, `Initialize`, `GetNextSoundSource` and the constructors of the 25 classes were decompiled and **read in full** (basis **R**; 67 functions). The receiving methods were identified by the virtual function tables compared in the earlier parts (`CPuppetEntity` 394 slots, `CLeggedPuppetEntity` 445, `CBaseWeaponEntity`, `CStaticModelEntity` 117): the slot an event calls carries the exported name of the method.
- **Member tables.** The binary declares member tables for 7 of the classes (`SAnimEventStep`, `SPuppetFireAnimEvent`, `SMeleeDamageAnimEvent`, `SPuppetReleaseSpawneeAnimEvent`, `SPuppetSoundAnimEvent`, `SSoundAnimEventToTerminate`, `SEffectAnimEventToTerminate`); names and parent were read, offsets only to order the members (not recorded).
- **Data.** `tools/re/helper_classes_verify.py` section 2 (about 2 minutes) walks the 88 `.ans` files and the models that embed an animation set (465 animations with an event holder, 1,313 events, 23 event classes) and prints per class the number of events, the flag values, the "first loop only" count and the values of the class's own member; section 1 lists the type tables.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run, so no event was seen firing; the engine side (`Engine.dll`: when `Execute` is called, what its two numbers are, how looping and `ae_bOnlyInFirstLoop` act); the base events `CSamSoundAnimEvent`, `CSamEffectAnimEvent`, `CSamShakeEffectAnimEvent`, `CSamMacroEvent`, `CRenderMuzzleFlashAnimEvent`, `CStandAloneSimpleEffectAnimEvent`, `CPlayerWeaponEffectForOneHandAnimEvent` (7 classes of 341 events that are in the data but not exported by this DLL: `T01-207`).

## Layout / rules

### 1. The event protocol

Every event object starts with the three members of `CAnimEvent` (`ans.md`): `ae_fFrame` (float frame), `ae_ulFlags` (an `AnimEventFlags` enumeration, 4 bytes) and `ae_bOnlyInFirstLoop` (boolean); the class's own members follow (identifier or period). The engine calls the event's virtual `Execute(handler, a, b)` when the animation passes the frame; `a` and `b` are two 4-byte values (**low**: `b` is passed unchanged to the tool operations (draw, store, attach, detach) and `a` is not used by any of the 14 `Execute` functions; they are probably the animation instance and the time or event handle, `T01-207`). `GetDescription` (or `mdGetName` for two classes) returns the text the editor shows for the event; the texts are in the table. An **identifier** member is an engine identifier (an integer produced from a name: `strConvertStringToID`); the default of every identifier member is the **invalid identifier**.

An event's **handler** is the object that owns the animation playback: it holds the **owner entity** (a handle at its first member, `saeh_henEntity` in the data). A game event first checks the **kind of its handler** (derived classes count) and then the **kind of the owner entity**, then calls the owner.

### 2. The 18 event classes

"Data" = events of that class in the 88 `.ans` files and the models with an embedded animation set (script, section 2); the saved name is the `S` twin where marked. "Call" = the method the event calls on its owner (virtual function of the owner's class unless marked non-virtual); "needs" = the handler and owner kinds it insists on; the message is the console line printed when they do not match (quoted from the binary, minus the trailing newline).

| Class (saved name) | Own members (default) | Description text | Needs; call | Data |
|---|---|---|---|---|
| `CAnimEventStep` (`SAnimEventStep`) | `aes_idAttachment` (invalid id) | `Step "<id>"` | puppet handler, **legged** owner: `OnAnimEventStep(id)` (a footstep: the attachment that steps; `puppet-legged-classes.md`). Messages: "Step event can be started only on puppet entities." / "... only on legged puppet entities." | 99 events, all flags 0; 18 files |
| `CAttachToolAnimEvent` | `atae_fLerpPeriod` (**1.0**) | none | puppet: `OnAnimEventAttachToolFromEntity(period, b)`. "Tool attaching animation event can be started only on puppet entities." | 1 event, period 0.4 |
| `CDetachToolAnimEvent` | none | none | puppet: `OnAnimEventDetachTool(b)`. "Tool detaching ..." | 4 events (flags 0, 0, 0, 2) |
| `CDrawToolAnimEvent` | `dtae_fDrawingPeriod` (**0.5**) | none | puppet: `OnAnimEventDrawTool(period, b)`. "Draw tool animation event ..." | 9 events; period 0.1 (6), 0.2 (2), 0.5 (1); flags 0 (2), 2 (7) |
| `CStoreToolAnimEvent` | `stae_fStoringPeriod` (**0.5**) | none | puppet: `OnAnimEventStoreTool(period, b)`. "Store tool animation event ..." | 7 events; period 0.1 (6), 0.2 (1); flags all 4 |
| `CHideAttachmentAnimEvent` | `haae_idAttachment` | none | puppet: `OnAnimEventHideAttachment(id)`. "Hide attachment animation event be started only on puppet entities." | 3 events |
| `CShowAttachmentAnimEvent` | `saae_idAttachment` | none | puppet: the show-attachment method (`macShowAttachment` / `OnAnimEventShowAttachment`)`(id)`. "Show attachment animation event be started ..." | 3 events |
| `CHidePlayerWeaponAttachmentAnimEvent` | `hpwaae_idAttachment` | `Hide attachment (<id>) on player weapon` | **weapon** handler, **weapon** owner (`CBaseWeaponEntity`): `OnAnimEventHidePlayerWeaponAttachment(id)` (non-virtual). "Hide player weapon attachment animation event can be started only on weapon entities." | 44 events; flags 0 (35), 1 (9) |
| `CShowPlayerWeaponAttachmentAnimEvent` | `spwaae_idAttachment` | `Show attachment (<id>) on player weapon` | as above: `OnAnimEventShowPlayerWeaponAttachment(id)` | 26 events; flags 0 (23), 1 (3) |
| `CMeleeDamageAnimEvent` (`SMeleeDamageAnimEvent`) | `mdae_idBlast` | `Melee blast ("<id>")` | puppet: `OnAnimEventMeleeDamage(blast id)`; no console message (**R**: the check is silent) | 36 events; flags 0 (35), 1 (1) |
| `CPuppetFireAnimEvent` (`SPuppetFireAnimEvent`) | `pfae_idBlast` | `Shoot blast (<id>)` | puppet: `OnAnimEventShootingBlast(blast id)`. "Puppet shooting blast animation event ..." | 129 events, all flags 0 |
| `CPuppetReleaseSpawneeAnimEvent` (`SPuppetReleaseSpawneeAnimEvent`) | `psae_idAttachment` | `Release spawnee ("<id>")` | puppet: `OnAnimEventReleaseSpawnee(attachment id)` (a spawner puppet releases the creature it carries). "Puppet release spawnee animation event ..." | 9 events; flags 0 (7), 1 (2); first loop only 2 |
| `CPuppetSoundAnimEvent` (`SPuppetSoundAnimEvent`, parent `CSamSoundAnimEvent`) | `psae_psstSoundSourceType` (enumeration `PuppetSoundSourceType`) | inherited | **no `Execute` of its own**: the base sound event asks the handler's `ProduceSound` (section 3); the member selects which of the puppet's sound sources plays (values in section 3) | 575 events |
| `CPuppetAppearAnimEvent` | none | none | puppet: `OnAnimEventAppear()` (non-virtual; an **empty** function in `puppet-entity-methods.md`). "Appear animation event ..." | **none in the data** |
| `CPuppetDisappearAnimEvent` | none | none | puppet: `OnAnimEventDisappear()` (non-virtual, empty). "Disappear animation event ..." | 1 event |
| `CPuppetExplodeAnimEvent` | none | none | puppet: `DoExplode()` (starts the explosion state, `puppet-entity-methods.md`). "Explode tool animation event ..." (sic) | **none in the data** |
| `CRagdollAnimEvent` (`SRagdollAnimEvent`) | none | `Convert to ragdoll` | puppet handler (silent if the owner is missing), **legged** owner: `OnAnimEventRagdoll()`. "Ragdoll event can be started only on legged puppet entities." | 20 events; flags 0 (13), 1 (7); first loop only 7 |
| `CThrowDeathToolsAnimEvent` | none | none | puppet: `OnAnimEventDeathThrowTools()` (non-virtual: throws the tools the dying puppet carries). "Death tool throw event ..." | 6 events |

Notes (**medium**): (1) The 17 `Execute` functions all return 0; a return value matters only for the base sound and effect events (section 4). (2) `CAttachToolAnimEvent`, `CDrawToolAnimEvent` and `CStoreToolAnimEvent` have a default period in the constructor (1.0, 0.5, 0.5 seconds); the data's periods are mostly 0.1 (one draw event keeps the default 0.5, no store event does) and the one attach event has 0.4. (3) The saved names are `S` twins for 6 classes and the exported `C` name for 10 (script). (4) `CPuppetFireAnimEvent` and `CMeleeDamageAnimEvent` carry a **blast identifier**: the name of an entry of the puppet's blast list (`puppet-entity-classes.md`, attacks; the identifiers are integers in the file and are not turned back into names here). (5) The event classes the data has but this DLL does not export (7 classes, 341 events) are the base sound, effect, shake-effect, macro and muzzle-flash events and two more; they are not in this DLL's exports, `T01-207`.

### 3. The flag word, the sound source type and the events' counts

`ae_ulFlags` (`AnimEventFlags`): over all 1,313 events the script counts 0 (1,189), 1 (92), 2 (19), 3 (3) and 4 (10). `ans.md` listed 0x0, 0x1 and 0x4 only; the draw and detach events (and 2 sound events) also carry **2** and 3 sound events **3**, so the field is a bit set with at least the bits 1, 2 and 4 (**medium**: 3 = 1 + 2). What each bit means is not read (`T01-207`); the data shows the bit **1** on 9 hide-attachment, 3 show-attachment and 7 ragdoll events and on one melee event, and the bit **4** only on the 7 store-tool events. `ae_bOnlyInFirstLoop` is 1 on 5 sound events, 2 spawnee events and 7 ragdoll events and 0 on all others of these classes.

`PuppetSoundSourceType` (the member of the puppet sound event), over the 575 events: 0 in 172, 1 in 43, 2 in 356; **4 events carry other values** (0x746e in 3 and 0x65000074 in 1: they look like text bytes, so those four are probably decoded from a record of another layout; `ans.md` also reports 4 events outside their animation's frame range; whether they are the same four was not checked). The enumeration's names were not recovered (`T01-207`).

### 4. Handlers

The four handlers share their first function: the **constructor calls one shared base constructor** (the `CSamAnimEventHandler` constructor, the same machine code for all four) and sets the class's table. The handler's own state is the base's: the **owner entity handle** at the first member and the time of the last evaluation. They export the same five functions (**R**, **medium**), which check the event's kind and call the owner:

| Function | What it does |
|---|---|
| `ProduceSound(event, b)` | passes the sound event to the owner's `OnAnimEventProduceSound(event, b)`; returns the owner's answer |
| `TerminateSound(id)` | passes the stop request to the owner's `OnAnimEventTerminateSound` |
| `ProduceEffect(event, b)` | an event that is **not an effect event** prints "Invalid animation event occured. ... can only recognize "effect event" for playing effects." and returns 0; otherwise the owner's `OnAnimEventProduceEffect(event, b)` and its answer |
| `TerminateEffect(id)` | the owner's `OnAnimEventTerminateEffect` |
| `ProduceShakeEffect(event)` | a non-shake event prints the matching message and returns 0; otherwise the owner's `OnAnimEventProduceShakeEffect(event)` and its answer |

- **`CPuppetAnimEventHandler`** (12 functions; data `SPuppetAnimEventHandler`, 26 `.wld`): owner `CPuppetEntity` (the five functions of `puppet-entity-methods.md`: `OnAnimEventProduceSound`, `...ProduceEffect`, `...ProduceShakeEffect`, `...TerminateSound`, `...TerminateEffect`). Messages say "Puppets can only recognize ...".
- **`CStaticModelAnimEventHandler`** (12; data `SStaticModelAnimEventHandler`, 9 `.wld`): owner `CStaticModelEntity` (the five functions `OnAnimEventProduceSound` ... of the static model, slots 109 to 113 of its table). Messages say "Static model(s) can only recognize ...". The sound event's kind is also checked in `ProduceSound` (the weapon handler does the same; the puppet handler checks only the effect kinds).
- **`CWeaponAnimEventHandler`** (11 = 7 + 4; **not in the data**): owner `CBaseWeaponEntity` (`OnAnimEventProduceSound`, `...ProduceEffect`, `...TerminateSound`, `...TerminateEffect`); **no shake effect**: a weapon cannot shake the camera through an event (the base handler's default is used). Messages say "Weapons can only recognize ...". The weapon handler is the one the two player-weapon attachment events insist on (section 2).
- **`CDoorAnimEventHandler`** (9 = 7 + 2; data `CDoorAnimEventHandler`, 33 `.wld`): sounds only. The owner is a `CDoorEntity`; `ProduceSound` hands the event to the engine's helper that plays an animation sound on the **door's sound source** with the door's sound parameters, and, when the played sound has bit 0 set in its flags (the sound is to be stopped later), adds it to the door's list of sounds to terminate; the function returns that bit. `TerminateSound` stops the matching sounds in that list. The door has no effect or shake handling here.

Whether the answer of `Produce...` (nonzero = "remember to terminate") controls the engine's later `Terminate...` calls is **low** (inferred from the door).

### 5. `CStaticModelAnimEventsData`

(9 functions: 7 standard + `Initialize` and `GetNextSoundSource`; data `SStaticModelAnimEventsData`, **10 members**, 9 `.wld`, saved once per static model entity that animates; **R**, **medium**.) The static model entity creates it **lazily** the first time its animation evaluation is enabled (`CStaticModelEntity::EnableAnimationEvaluation`, read): the constructor makes a `CStaticModelAnimEventHandler`, **four sound sources** and two empty lists. The member order of the data equals the constructor's order (script and constructor: checked member by member, **high**):

| # | Member (data) | Meaning |
|---|---|---|
| 1 | `smaed_hsmaehAnimEventHandler` | the handler (a handle), its owner set by `Initialize` |
| 2 to 5 | `smaed_hss0`, `smaed_hss1`, `smaed_hss2`, `smaed_hss3` | four sound sources (handles) |
| 6 | `smaed_iLastSoundSrc` | the ring counter (0 to 3) of the next source to hand out |
| 7 | `smaed_csoSoundAnimEventsToTerminate` | list of `CSoundAnimEventToTerminate` records (sounds that must be stopped) |
| 8 | `smaed_csoEffectAnimEventsToTerminate` | list of `CEffectAnimEventToTerminate` records |
| 9 | `smaed_fSoundHotSpot` | the sounds' hot-spot distance |
| 10 | `smaed_fSoundFallOff` | the sounds' fall-off distance |

`Initialize(a, b, c, d, e)` stores the hot spot and fall-off (the last two arguments), makes the entity the handler's owner and initialises the four sound sources from the other arguments (one sound source setup per source). `GetNextSoundSource` returns the sources in the order **`hss1`, `hss2`, `hss3`, `hss0`, `hss1`, ...** (the counter starts at 0, a value 0, 1, 2 selects `hss1`, `hss2`, `hss3`, any other `hss0`; the counter then goes up by one modulo 4): a round robin so that the next sound does not cut the previous one (**medium**, the order read from the function). 

### 6. The terminate records

`CSoundAnimEventToTerminate` (data table `SSoundAnimEventToTerminate`: `saett_hssSoundSource`, `saett_ulEvent`) and `CEffectAnimEventToTerminate` (`SEffectAnimEventToTerminate`: `eaett_hpfxen`, `eaett_ulEvent`; parent `CScriptObject` in both tables): a record of **one started sound or effect and the event that started it**, kept in the static model's two lists so that the matching `Terminate...` call can find it. `hss` is a handle to a sound source, `hpfxen` a handle to a particle effect entity (names; **medium**), `ulEvent` the event identifier (4 bytes). Neither class is in the data; their constructors set the first member to 0.

## Invariants

1. All 18 event classes have 7 to 9 exported functions and **never more than two named methods**; 17 of 18 have `Execute` (the exception is the puppet sound event, which uses its base).
2. The 16 data event classes derive from `CAnimEvent` (15) or `CSamSoundAnimEvent` (1): 16 of 16 (type tables).
3. The event objects of the 16 classes number 972 of the 1,313 events of the install; the other 341 belong to 7 classes this DLL does not export (script).
4. `SStaticModelAnimEventsData` has the 10 members listed in section 5 in one variant in all 9 files (script, section 1); their order equals the order of the constructor's stores.
5. Every `Execute` returns 0 (17 of 17, read).

## Open questions

1. **The engine side of animation events**: when `Execute` is called, the meaning of its two arguments, the effect of looping and `ae_bOnlyInFirstLoop`, the bits of `AnimEventFlags`, the names of `PuppetSoundSourceType`, which machine evaluates events in a network game, and the seven base events the data uses: **T01-207**.
2. The numbers behind the blast identifiers of the fire and melee events (which blast entry of which puppet): `puppet-entity-classes.md` T01-143; the list of attachments an identifier names: **T01-207**.
3. The two numbers `Initialize` receives and the exact sound source setup: **T01-219**.

## Test vectors

1. A `SPuppetFireAnimEvent` with `pfae_idBlast` = identifier X on a puppet handler whose owner is a puppet: `Execute` calls the owner's shooting-blast method with X and returns 0. On a weapon handler nothing happens beyond the message "Puppet shooting blast animation event can be started only on puppet entities." (the first check fails; return 0).
2. A `CDrawToolAnimEvent` created in the editor has period 0.5; in the retail files its period is 0.1 for 6 of 9 events.
3. `GetNextSoundSource` with `smaed_iLastSoundSrc` = 3: returns `hss0` and the counter becomes 0; with the counter 0 it returns `hss1` and the counter becomes 1.
