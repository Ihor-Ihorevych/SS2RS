# Character tools of puppets: desired and attached tools, the tool state machine (T01-144)

- **Roadmap task:** T01-144 (first of two pages; carried objects, rides and seats are in [`puppet-carry-rides.md`](puppet-carry-rides.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** only for what `tools/re/tools_state_verify.py` reproduces from the data (the 68 tool files, the 2 files that declare shields); **medium** for every rule read from `Sam2Game.dll` (the state machine, the update rules, the net rules), because each rests on **one reading** of the decompiled functions of the class and **nothing was run**; **medium-low** for the items marked so; **low** for the open questions.

Prerequisites: [`puppet-entity-classes.md`](puppet-entity-classes.md) (members, the step), [`puppet-classes.md`](puppet-classes.md) (3.3: the `CTA_` and `TDT_` numbers), [`mounted-tools.md`](mounted-tools.md) (mounted tools, who mounts them), [`puppet-attacks.md`](puppet-attacks.md) (8: the launch of a thrown tool, **not repeated**), `rsc-ctl-schema.md` (the `.ctl` members), [`main-loop.md`](main-loop.md) (the clock).

## Summary

1. A **character tool** (`CCharacterTool`, one `.ctl` file; 68 in the install) is anything a character holds in a limb: every weapon of the player and the NPCs (one tool per hand), the objects a character picks up (health packs, a ball, a club) and the boss's thrown rocks. A puppet keeps two lists: the **desired tools** (what the game wants it to hold) and the **attached tools** (what it holds now, each with a small state machine). **`UpdateTools` turns the first list into the second, `HandleToolsActions` runs the state machine**, both once per step on every machine.
2. The desired list is changed by `DrawTool` (hold a tool), `StoreTool` (put one away), `PickTool` (pick up an entity as a tool) and `ThrowTool`, by the script macros of the same names and by the player's weapon change. **Two tools never share a limb**: a desired tool displaces the attached tool of the same limb (section 3).
3. The attached tool's **state** is the enumeration `CharacterToolAction` (`CTA_`, 15 values): drawing (0-3), storing (4-7), picking (8-11), throwing (12-14). Animation events of the model (draw, store, attach tool from entity, detach tool) move the states 1, 5, 9 and 13 on; the resize and lerp states (2, 6, 10) last the event's duration; when a tool reaches 3, 7, 11 or 14 the puppet receives the script event **`ToolDrawn`**, **`ToolStored`**, **`ToolPicked`** or **`ToolThrown`**.
4. A tool is **hit-proof armour** when the puppet's parameters list it as a **shield** for a body part (`pbp_asdShields`): damage to that part is absorbed by the tool's health and the tool is thrown off when the health is gone (section 6; 2 files: the Neanderthal's shield, the pyramid boss's doors).
5. **Riders:** a rider that cannot fire from its own weapons has its desired tools stored when it boards and gets them back when it leaves (section 7).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Read in full (decompiled once each): `CPuppetEntity::UpdateTools`, `HandleToolsActions`, `AddDesiredTool`, `RemoveDesiredTool`, `IsToolDesired`, `IsToolAttached`, `GetDesiredToolOnLimb`, `GetAttachedToolOnLimb`, `GetAttachedTool`, `GetAttachedToolByID`, `IsToolFromEntityAttached`, `GetToolAttachedFromEntity`, `AttachTool`, `DetachTool`, `StartDrawingTool`, `DrawTool`, `StartStoringTool`, `StoreTool`, `StartPickingTool`, `PickTool`, `StartThrowingTool`, `ThrowTool`, `ConnectLastAnimToTool`, `ClearToolAnimRelation`, `GetToolRelatedWithAnim`, `OnAnimEventDrawTool`, `OnAnimEventStoreTool`, `OnAnimEventDetachTool`, `OnAnimEventAttachToolFromEntity`, `OnAnimEventDeathThrowTools`, `DeleteAttachedTools`, `ThrowDeathTools`, `StoreToolsBeforeRide`, `RestoreToolsAfterRide`, `macDrawTool`, `macStoreTool`, `macPickTool`, `macThrowTool`, `macThrowToolAtTargetEntity`, `macThrowPickedTool`, `macGetLastThrownTool`, `ThrowPickedTool`, `PreSendUpdate`, `PostReceiveUpdate` (the tool part), `OnStep`, `OnReceiveDamage` (the shield part), `OnShutDown`, `CPlayerPuppetEntity::SetCurrentWeapon`, `PutDownWeapons`, `RestoreToolsAfterRide`. The member names of the two records come from the DLL's member tables (names and order; offsets are not recorded): `CDesiredCharacterTool` (3) and `CAttachedCharacterTool` (13). Callers of the tool functions were listed over the whole DLL (direct calls) and the virtual-call sites of the relevant table positions scanned.
- **Data.** `python -I tools/re/tools_state_verify.py "<install>"` (about 1 minute) prints sections 1 to 6 quoted below.
- **Not checked.** Nothing was run: no timing of any state was measured. The **durations** of the resize states are the lengths carried by the model's animation events, which live in the `.mdl` files (T01-320); the **view of the tool in the player's hand** (weapon models, `weapon-view.md`) and the **launch maths** (`puppet-attacks.md` 8) are other pages. The `Tool*` script events were seen only as strings sent by the functions above.

## Layout / rules

### 1. The two records (**high** for names and order, **medium** for meanings)

**Desired tool** (`CDesiredCharacterTool`, 3 members, in the order): `dct_pctTool` (the tool), `dct_henPickedEntity` (an entity to pick up as the tool, or none), `dct_bImmediatelly` (do it **without animations**).

**Attached tool** (`CAttachedCharacterTool`, 13 members): `act_pctTool` (the tool), `act_idTool` (an identifier made from the tool: it names the dynamic child of the puppet's model that shows the tool, and is what the shield list refers to), `act_henPickedEntity`, `act_ctaAction` (the state, `CTA_`), `act_tdtDetachType` (`ToolDetachType`: 0 `TDT_AUTO`, 1 `TDT_STORE`, 2 `TDT_THROW`), `act_tmActionStarted` (time the current resize or lerp state began), `act_fActionLength` (seconds), `act_vFullToolStretch` (the tool's final scale, 3 floats), `act_idThrowingDescription` and `act_henTargetEntity` (set by `ThrowTool`, `puppet-attacks.md` 8), `act_qvInitialTargetOffset` and `act_qvFinalTargetOffset` (the start and the end placement of a picked object in the hand, section 4) and `act_slHealth` (copied from the tool's `ct_slHealth` when it is attached: **-1** in 66 tool files, **40** in two).

Other members of the puppet (`puppet-entity-classes.md` 3): `en_aactAttachedTools` (the attached list), `en_adctDesiredCharacterTools`, `en_adctStoredToolsBeforeRiding`, `en_atarToolAnimRelations` (pairs of an animation identifier and a tool: see 2.2), `en_henToolEntityToPick`, `en_henLastThrownToolEntity`. Both lists have **no fixed size**; the data shows 0 to 2 entries in a stored puppet (world data).

### 2. The desired list

**2.1 `AddDesiredTool(tool, immediately, pickedEntity)`** (a null tool is ignored; **medium**): walk the list; if an entry already has **the same tool and the same picked entity**, nothing happens (the flag of the old entry is **not** updated); otherwise **if an entry of the same limb exists** (`ct_idLimb` equal) it is **overwritten** with the new tool, entity and flag; otherwise the tool is appended. So the list holds **at most one tool per limb**, and the entry for a limb keeps its position. The limb is the tool's identifier `ct_idLimb`: in the data `RightHand` (37 tools), `LeftHand` (22), `Hands` (1: `genericentity`) and the **empty identifier (8 tools)**; the eight tools with an empty limb all compete for **one** "empty" limb (`crossbow_arrow`, `kwongo_rock`, the five `kwongo_skyscraper*` and `kwongocatapultprojectile`: thrown projectiles and the boss's objects).

**2.2 Callers.** `DrawTool(tool, immediately)` and `macDrawTool(tool)` (flag **0**) add a tool with no entity; **`PickTool(entity, tool, immediately)`** (script macro `macPickTool`, flag default 0) requires the entity to answer yes to "can be picked as a tool" (a virtual question of the entity), takes the **tool from the argument or, if none, from the entity** (a second virtual question), refuses a tool whose pick animation is invalid with the warning `Invalid picking animation specified for tool "<name>".`, stores the entity in `en_henToolEntityToPick` and adds the tool with the entity. `MountDefaultTools` adds the mounted tools (`mounted-tools.md` 2) with flag **1** (immediately). The player's `SetCurrentWeapon` calls `DrawTool(weapon's tool for the hand, flag from the caller)` (`weapon-view.md`). **`StoreTool(tool, immediately)`** (a remote call, run on **every** machine, host first; `macStoreTool`): **removes** the tool from the desired list, and with the flag set **detaches it at once** (no animation). `RemoveDesiredTool` removes by tool: the last entry takes the place of the removed one (order not kept). `ThrowTool` is `puppet-attacks.md` 8 (it also removes the tool from the list).

### 3. `UpdateTools`: desired list to attached list (**medium**)

Run every step, **first loop over the desired tools**, in list order. For a desired tool that is **not attached** (the tool object itself is compared), with `X` = the attached tool of the same limb (none, or another tool or the same tool with another entity):

| Flag | `X` | Result |
|---|---|---|
| not immediate | none | attach the tool; the state starts at **0 `CTA_READY_FOR_DRAWING`**, or at **8 `CTA_READY_FOR_PICKING`** when a (live) picked entity is given |
| not immediate | exists and differs in tool **or** entity | if `X` is in state **3 (drawn)**: `X` is told to leave: state **12** when its detach type is `TDT_THROW`, else state **4** when its detach type is `TDT_STORE` **or this machine is not a client**, else (a client, `TDT_AUTO`) **`X` is detached at once**; if `X` is in state **11 (picked)**: state **12**. In every other state of `X` nothing is done (it finishes what it is doing). The new tool waits: it is attached in a **later step**, once the limb is free |
| immediate | none | attach the tool in state **3 (drawn)** or, with a live entity, **11 (picked)** |
| immediate | another tool | **detach `X` at once**, attach the new one in state 3 (or 11) |

**Second loop over the attached tools**, in list order: an attached tool that is **not desired** and whose limb has **no desired tool** leaves in the same way as `X` above (3 → 12 or 4 or detached; 11 → 12). (The loops begin by replacing a tool object by its reloaded copy if the resource system flags it; ignorable.)

Consequences (**medium**): a weapon change on a host is a **store animation of the old tool, then a draw animation of the new one**; a client with `TDT_AUTO` (the only detach type any function read ever sets besides 2: open question 1) **skips the store animation** of the old tool. A picked object that is no longer wanted is **thrown** (12), never "stored".

### 4. `AttachTool`, `DetachTool` and `HandleToolsActions` (**medium**)

**`AttachTool(tool, state, pickedEntity)`** (returns nothing useful; refuses a null tool or a puppet without a model instance): creates a **dynamic child** of the puppet's model instance named by `act_idTool`. Its **target** is the tool's attachment: when `ct_idAttachment` is the invalid identifier the child targets the bone `ct_idTargetBone` with the placement `ct_qvTargetOffset`; otherwise the model configuration's child of that name is looked up in the puppet's model and its target and offset are copied (a missing one leaves the child untouched). Its **model** is the tool's `ct_pmcModel`, or, with a live picked entity, **the entity's own model** (an entity without a model instance makes the attach fail). The **full stretch** is `ct_fStretch` on all axes when `ct_bStretchWithOwner` is set (67 of 68 tools) and otherwise `ct_fStretch` divided per axis by the owner's current stretch (the tool then keeps its absolute size; 1 tool). The **initial stretch** is **zero** when the state is 0 and the puppet's model has the tool's draw animation (the tool grows during state 2) and the full stretch otherwise. The record gets the tool, `act_idTool`, the entity, the state, the full stretch and the **health copied from `ct_slHealth`**; then the picked entity (if any) is told it was picked (the entity's virtual `OnPickedAsTool`, `worldcontent-classes.md`; what it does belongs to the entity class) and **the puppet's brain is told `OnToolPicked`** (**medium-low**: the decompiled function ends with this call and no condition on the state; the brain turns it into an event, `mounted-tools.md` 3). So the brain is notified at **every** attach, not only for picked objects (open question 1).

**`DetachTool(tool)`**: if attached and the puppet has a model: deletes the dynamic child and removes the record by moving the **last record into its place** (order not kept) and decrementing the count.

**`HandleToolsActions`** walks the attached list in order, **once per step**; time is the 64-bit clock of `main-loop.md` (2^32 ticks per second), `done` means **`now` is strictly after `start + length`**, `elapsed` is `now - start`. Per state:

| State | `CTA_` name | Action each step |
|---|---|---|
| 0 | `READY_FOR_DRAWING` | `StartDrawingTool` (below); the state leaves 0 in the same call |
| 1 | `DRAWING` | nothing: waits for the **draw animation event** (`OnAnimEventDrawTool(length, animation)`: finds the tool related to that animation, sets state **2**, `act_tmActionStarted` := now, `act_fActionLength` := the event's length, then clears the relation) |
| 2 | `DRAWING_AND_RESIZING` | the child's stretch := `min(elapsed / length, 1)` × full stretch (**1** when the length is not above 0); when `done`: state **3** and the event **`ToolDrawn`** |
| 3 | `DRAWN` | nothing (the resting state of a held tool) |
| 4 | `READY_FOR_STORING` | `StartStoringTool` |
| 5 | `STORING` | nothing: waits for the **store animation event** (`OnAnimEventStoreTool`: same as the draw one, state **6**) |
| 6 | `STORING_AND_RESIZING` | stretch := `max(1 − elapsed / length, 0)` × full stretch (the factor is **0** when the length is not above 0); when `done`: **detach the tool**, event **`ToolStored`**, and **the loop ends for this step** (the list changed) |
| 7 | `STORED` | detach the tool, event **`ToolStored`**, end of the step's loop |
| 8 | `READY_FOR_PICKING` | `StartPickingTool` |
| 9 | `PICKING` | nothing: waits for the **"attach tool from entity" animation event** (`OnAnimEventAttachToolFromEntity`, below) |
| 10 | `PICKING_AND_LERPING` | the child's offset := interpolation of `act_qvInitialTargetOffset` to `act_qvFinalTargetOffset` by `t = min(elapsed / length, 1)` (**1** when the length is not above 0): position linear; orientation: the end quaternion is negated when the dot product is negative, then **normalized linear interpolation without renormalizing when the dot product is at least 0.99**, otherwise spherical interpolation. When `done`: state **11** and the event **`ToolPicked`** |
| 11 | `PICKED` | nothing (held; leaves only through `UpdateTools`: 12) |
| 12 | `READY_FOR_THROWING` | `StartThrowingTool` |
| 13 | `THROWING` | nothing: waits for the **detach tool animation event** (`OnAnimEventDetachTool`: `LaunchTool` for the related tool, `puppet-attacks.md` 8) |
| 14 | `THROWN` | detach the tool, event **`ToolThrown`**, end of the step's loop |

The events are sent to the puppet through the script event system (`macSendEvent`), so a macro of the level can wait for them. **The loop ends early** after a detach in states 6, 7 and 14, so with two tools leaving in the same step the second is handled in the **next** step.

**The `Start...` functions** (all: tool must be attached; the tool's animation term is mapped through the puppet's animation scheme (`ans.md`) to an animation of its model, and the animation must contain the matching **animation event class**):

| Function | Animation (tool member) | Event class searched | Lerp flag | Playing | Next state | If the animation or event is missing |
|---|---|---|---|---|---|---|
| `StartDrawingTool` | `ct_idDrawAnimationTerm` | draw-tool event | `ct_bLerpDrawAnimation` | normal | **1** | state **3** at once (no `ToolDrawn` event, no resize) |
| `StartStoringTool` | `ct_idStoreAnimationTerm`; **if it is empty, the draw animation `ct_idDrawAnimationTerm` played in reverse** | store-tool event | `ct_bLerpStoreAnimation` | normal or reverse | **5** | state **7** at once |
| `StartPickingTool` | `ct_idPickAnimationTerm` | attach-tool event | `ct_bLerpPickAnimation` | normal | **9** | the tool is **removed from the desired list** and the state is set to **14 (thrown)**: the next step detaches it and sends **`ToolThrown`** |
| `StartThrowingTool` | `ct_idThrowAnimationTerm` | detach-tool event | `ct_bLerpThrowAnimation` | normal | **13** | `LaunchTool` at once (`puppet-attacks.md` 8: it sets 14) |

Playing means: a cloned animation state is started with a blend time of **0 when the lerp flag is clear and `en_fAnimLerpTime` when it is set**, the animation is played once (not looping) and **the animation identifier is recorded together with the tool** (`ConnectLastAnimToTool`: appended to `en_atarToolAnimRelations`, 8 bytes per pair) so that the later animation event, which names only the animation, finds its tool. `GetToolRelatedWithAnim` returns the **first** pair with that animation; the three `OnAnimEvent` handlers delete the pair they used (the last pair moves into its place). An event whose animation has no pair (for example after the puppet was reset) does nothing.

**`OnAnimEventAttachToolFromEntity(length, animation)`** (the picking event; **medium-low**, the longest function of the group): requires `en_henToolEntityToPick`, its model instance and configuration, the puppet's model and the tool related to the animation; the new target is computed **in the bone frame of the tool's target bone**: the entity's current world placement is converted to the **bone-relative** placement (inverse of the bone's absolute placement), normalized, and stored as the **start** offset; the tool's own target offset (`ct_qvTargetOffset`, or the one copied from the model's child, as in `AttachTool`) is the **end** offset; the child's model becomes the entity's model with the **entity's stretch divided by the puppet's stretch**; the attached tool gets the entity, state **10**, the start time, the event's length and the two offsets; the entity is told it was picked (virtual call) and the relation is cleared. So a picked object **flies from where it lies into the hand** during `length` seconds.

**Death.** `OnAnimEventDeathThrowTools` (a death-animation event) calls `ThrowDeathTools` **only when the puppet is not alive** (the virtual `IsAlive`). `ThrowDeathTools` walks the attached list once, and for every tool whose **`ct_bThrownWhenOwnerDies` is set** (13 of 68: `extralife_l`, `genericentity`, `health_large_l`, `health_medium_l`, `neanderthalclub`, `neanderthalshield`, `npc_bazooka_r`, `npc_doubleshotgun_item`, `npc_seriousjump_item`, `npc_shotgun_r`, `npc_zapgun_l`, `npc_zapgun_r`, `shells_l`) removes it from the desired list, **launches it with the random launch** (`LaunchTool`, no target) and detaches it. Because `DetachTool` moves the last record into the freed place while the loop index moves on, **with two such tools the record that took the place of the first is skipped** (**medium**: the listed behaviour of the two functions together; it matters only for a puppet that carries two of those tools: `neanderthal.ep` holds a club and a shield). `OnShutDown` and `OnDelete` delete all dynamic children and clear both lists and the replicated copies; `DeleteAttachedTools` is the plain clear of the attached list and the children.

### 5. Network (**medium**)

- `UpdateTools` and `HandleToolsActions` run **on every machine** and both lists are local. Only the **first two desired tools** are replicated: `PreSendUpdate` clears the replicated members and copies list entries 0 and 1 (tool, picked entity, immediate flag) to `en_pctFirstDesiredTool`/`en_henPickedEntityOfFirstDesiredTool`/`en_bSetFirstDesiredToolImmediatelly` and the three second members; `PostReceiveUpdate` resizes the local list to the **number of non-null replicated tools** (0, 1 or 2; a null first tool ends the count at 0) and copies them back. A third desired tool is **never replicated**; the data has at most 2 (two weapons).
- `StoreTool` and `ThrowTool` are remote calls delivered to all machines. The events of `HandleToolsActions` (`ToolDrawn`...) are sent by every machine that runs the state machine; which machine runs the macro that waits for them was not checked.

### 6. Shields (`pbp_asdShields`, `OnReceiveDamage`; **medium**, data **high**)

`SPuppetBodyParams.pbp_asdShields` is a list of **`CShieldDescription`** records, each `sd_idShieldTool` (the identifier that equals `act_idTool` of the attached tool) and `sd_idShieldPart` (the name of a part of the puppet's mechanism). In the data (script section 5): `neanderthal.ep` has **one** (`NeanderthalShield`, part `L_Hand`); the pyramid boss `pyramidslow.ep` has **five**, all with an **empty tool identifier** (`Door_Left`, `Door_Right`, `Door_Up`, `Hole`, `Pyramid`); no other file has any.

In `OnReceiveDamage`, **after the invulnerability checks and before the damage is applied**: if the hit hull belongs to a mechanism part with a name, the list is searched for **that part** (first match):
1. **the tool identifier is the invalid identifier** (the pyramid): **the damage is ignored** (the function returns; the part is armoured for ever);
2. otherwise the **attached tool with that identifier** (`GetAttachedToolByID`) is looked up: **if it is attached**: when its health is **below 1** the damage is **ignored** (a tool with health -1 absorbs everything); otherwise **health := health − damage**; when the result is **below 1** the tool is **thrown** (`ThrowTool(tool, no description, no target)`: `puppet-attacks.md` 8) and **the whole damage is still absorbed** (nothing passes to the puppet), and the function returns;
3. **if the tool is not attached any more** (shield thrown off or never drawn) the damage goes on as usual.

So the Neanderthal's shield (health 40) soaks the first **40 damage** that hits the left hand (the amount of the damage record, before any later damage processing), then flies off; the club has health 40 too but is in no shield list. The tool health is **per attached record** (reset to the tool's 40 at each attach).

### 7. Riding (`StoreToolsBeforeRide`, `RestoreToolsAfterRide`; **medium**)

`RegisterForRide` calls the virtual `StoreToolsBeforeRide` **when the puppet cannot fire from its own weapons** (`puppet-carry-rides.md` 3.4); `UnregisterFromRide` always calls `RestoreToolsAfterRide`. The base `StoreToolsBeforeRide` **clears the stored list, copies into it every desired tool that has no picked entity** (a picked object is not kept), then **empties the desired list**; the base `RestoreToolsAfterRide` **copies the stored list back to the desired list** when it has entries and clears the stored list. After the first the next `UpdateTools` stores every held tool (3 → 4 animation, a client detaching at once); after the second the tools are drawn again. The **player** overrides both (`mounted-tools.md` 4: both weapons are put down, the selected weapons are chosen again; when the remembered right-hand weapon has index 11 the left hand is not restored; the meaning of that index was not resolved).

## Data (script)

From `python -I tools/re/tools_state_verify.py` (68 `.ctl` files, 126 puppet files read for section 5):

- **Animation terms empty** (the step is skipped, section 4): draw 24 of 68, **store 60 of 68**, pick 55 of 68, throw 47 of 68. So **36 tools store by playing the draw animation backwards** (store term empty, draw term set), **8 have a store term** and **24 have neither** (they store at once, state 7); 13 tools can be picked (`genericentity`, `health_large_l`, `health_medium_l`, the five `kwongo_skyscraper*`, `neanderthalclub`, `neanderthalshield`, `npc_doubleshotgun_item`, `npc_seriousjump_item`, `playerweapon_doubleshotgun_l`) and 21 have a throw animation.
- **Lerp flags**: draw 67 set, store 68, pick 68, throw 67. `ct_bStretchWithOwner` 67, `ct_bStraightenUpWhenThrowing` 9, `ct_bThrownWhenOwnerDies` 13, `ct_ptecToolEntityCreator` set in 17, model pointer null in 6, health -1 in 66 and 40 in 2 (`neanderthalclub`, `neanderthalshield`).
- **`ct_fStretch`** values: 0.3, 0.6098, 0.7 (2), 0.8 (2), 0.9 (6), **1.0 (36)**, 1.2 (2), 1.3, 1.5, 1.75 (4), 2.0 (3), 2.1 (2), 2.2 (2), 4.0 (2), 6.2 (2), 20.0.
- **Limbs**: right hand 37 tools, left hand 22, `Hands` 1, empty 8. The 22 left-hand tools are `playerweapon_*_l` (17), `chicheweapon_uzi_l`, `cutsequences_lollypop_l`, `neanderthalshield`, `npc_doubleshotgun_item` and `npc_seriousjump_item`.
- **Shields**: section 6. **Default riders**: one `CCharacterMount` in the puppet files (`albinowithchick.ep`, seat `Seat`, character parameters `HellChick.ep`; script section 6; `puppet-carry-rides.md` 4).

## Invariants

1. Every tool file has exactly one `CCharacterTool` (68 of 68); its limb is one of four values (script).
2. `AddDesiredTool` never lets two desired tools share a limb (a code rule, **medium**); `UpdateTools` never attaches a tool while the limb of an attached tool is still occupied by another tool in a non-leaving state (**medium**).
3. The states a tool reaches by itself are, in order: draw 0 → 1 → 2 → 3 (or 0 → 3); store 4 → 5 → 6 → detached (or 4 → 7 → detached); pick 8 → 9 → 10 → 11 (or 8 → 14 → detached); throw 12 → 13 → 14 → detached (or 12 → launched → 14). A tool detaches **only** from 6, 7 (done) and 14, from `StoreTool`/`UpdateTools` immediately, from a throw or from a puppet's removal (**medium**).
4. `ToolDrawn` is sent when a tool **arrives** in 3 through state 2, `ToolStored` when it leaves through 6 (done) or 7, `ToolPicked` when it arrives in 11 through state 10, `ToolThrown` from state 14. **No event** is sent for the shortcut 0 → 3 (no draw animation or event), for an immediate attach in 3 or 11, nor for a tool detached by `UpdateTools` of a client or by `StoreTool` with the flag (**medium**).

## Open questions

1. **Who sets `TDT_STORE` (1)**: no function read writes it; the field starts at the default (0) and `ThrowTool` writes 2. If it is never written, a client always detaches a no-longer-desired tool at once (no store animation on clients). Also the **brain notification** at every attach (4) and the **random launch of a death tool** need a run-time check: **T01-320** (with the animation-event data: which animations of which models carry the draw, store, attach and detach events, and their lengths, a data task on `.mdl` files).
2. **Order and timing in a real session** (two weapons drawn together, the dual-wield change, a picked object on a limb already holding a weapon): **T01-130** (instrumentation harness) and the player rules **T01-114**.
3. **`ConnectLastAnimToTool` when two tools start the same animation** (the first pair wins at the event): not exercised by the data (no two tools of one puppet share an animation term in the shipped weapons, not checked) - **T01-320**.

## Test vectors

1. **Weapon change on a host** (puppet holds `playerweapon_colt_r` in state 3, the game draws `playerweapon_uzi_r`, same limb, not immediate): step 1: the Uzi is not attached; the Colt (3) becomes 4; step 2: `StartStoringTool` (if the Colt's tool has no store term the draw animation is played backwards, provided the model's animation carries a store event; otherwise state 7 at once), state 5; the store event moves it to 6, shrinking over the event's length, then it is detached and **`ToolStored`** is sent; the next step attaches the Uzi in state 0 with stretch 0, `StartDrawingTool` plays its draw animation (state 1), the event gives state 2 growing the stretch to the full one, then **`ToolDrawn`**.
2. **Immediate draw** (`MountDefaultTools`, flag 1): the tool is attached directly in state 3 with its full stretch, no events.
3. **Shield:** `neanderthal.ep`, a hit of 25 to the left-hand hull: shield health 40 → 15, the puppet takes **no** damage; a second hit of 25: health −10, the shield is thrown (`ThrowTool`), the puppet takes **no** damage; a third hit: normal damage. A hit of 25 to the pyramid's `Door_Left` part: ignored.
4. **A client** with the shield tool no longer desired and detach type 0: the tool is detached at once with no event from `HandleToolsActions`.
