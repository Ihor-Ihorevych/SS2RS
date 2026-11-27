# Mounted tools and the use of throw descriptions (T01-123)

- **Roadmap task:** T01-123 (follow-up of `weapon-classes.md` open question 7; with `puppet-attacks.md` section 8, which specifies the maths of the throw)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/tools_throw_verify.py` reproduces from the data: the 10 mounted-tool records of 9 puppet files, which of them name the tool by pointer and which only by identifier, that the named `.ctl` files exist, the 30 throw descriptions of 15 files with their identifiers and speed windows, and the three behaviour files that name `ThrowItem`; **medium** for everything read from `Sam2Game.dll` (the mount routine and its resolution of the identifier, the tool-name table, the lookup of a throw description, the behaviour action, the player's handling of tools), because each rests on **one reading** of the decompiled functions and **nothing was run**; **low** for the items marked low and for "Open questions".

Prerequisites: `weapon-classes.md` (section 8 introduced `CMountedTool` and `CThrowDescription`), `puppet-attacks.md` (section 8 gives the graphs, units and the launch maths of a throw, section 9 the solvers: **not repeated here**), `puppet-entity-classes.md` (the tool members), `behavior-action-classes.md` (the action), `rsc-ctl.md` (the `.ctl` files), `players-session-classes.md` (the player).

## Summary

1. **A mounted tool is a character tool a puppet carries from its creation**: a record of an identifier and a pointer to a `.ctl` file, in the puppet's parameter file (`pp_amtMountedTools`, 0 to 2 per file, 10 records in 9 files: Neanderthals' clubs and shield, three NPC gunners and one plasma NPC) and, as a second list, in the puppet entity itself (`en_amtMountedTools`, empty in every stored puppet).
2. **The "obsolete identifier" is the first member of the record**: the data calls it `mt_idTool`, the binary `mt_idToolObsolete` (same position, offset 0 of the 8-byte record; `weapon-classes.md` section 8 listed it as a second, missing member: **corrected**). It names a tool by the **file name** of its `.ctl`; when the puppet mounts its tools the name is looked up in a table of all `.ctl` files under the `Tools` folder, the file is loaded into the pointer member and **the identifier is cleared** (section 2). 2 of the 10 records use only the identifier.
3. **A throw description is found by its identifier with a linear search** over the 24-byte records of the puppet's `pp_atdThrowDescs` (first match). The behaviour action `CBAThrowPickedTool` names one (`ThrowItem` in all three `.cb` files that have it) or none, in which case the tool is thrown with the random launch.
4. **The player has no tool code of its own**: its brain exports no tool function; the player puppet only **puts the weapons down before a ride and selects them again afterwards**, and treats each weapon's tool like any puppet's (store the old tool when a weapon is replaced).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Exported functions read: `CPuppetEntity::MountDefaultTools`, `GetThrowDescription`, `ThrowPickedTool`, `ThrowPickedToolAtTargetEntity`, `macThrowPickedTool`, `macThrowPickedToolAtTargetEntity`, `GetToolAttachedFromEntity`, `StoreToolsBeforeRide`, `RestoreToolsAfterRide`; `CPlayerPuppetEntity::StoreToolsBeforeRide`, `RestoreToolsAfterRide`, `SetNewWeaponToPlayerHand`; `CBrainEntity::OnToolPicked`; `CBAThrowPickedTool::Execute`, `GetDescription`; `CMountedTool::mdGetName`; `CThrowDescription::mdGetName`; the static initialiser that builds the tool-name table and the member tables of the two records. The member table of `CMountedTool` was read with the same scan as in `weapon-classes.md`. The dictionary's file search pattern and folder name are strings of the binary.
- **Data.** `tools/re/tools_throw_verify.py [DATA_DIR]` (about 3 minutes) prints the tables of sections 1 to 4 below.
- **Not checked:** nothing was run; the tool **state machine** (`CTA_`, attach, draw, store, detach, `UpdateTools`: T01-144), the animation events that trigger the throw, `AddDesiredTool` and `UpdateTools` in detail, the entity creators of thrown tools (T01-262).

## Layout / rules

### 1. `CMountedTool` (data: script section 1; **high**)

| Member (data name / binary name) | Type | Meaning | Values |
|---|---|---|---|
| first: `mt_idTool` / **`mt_idToolObsolete`** | `IDENT` | the tool's **file name** without folder and extension (the obsolete way to name a tool) | empty in 8 records; `NPC_Zapgun_R` (`simba_warrior.ep`), `NPC_Bazooka_R` (`zixie_gunner.ep`) |
| second: `mt_pctTool` | `Ptr<CCharacterTool>` | the tool | set in 8 records, null in the two with an identifier |

The 10 records: `neanderthal.ep` has two (`NeanderthalShield.ctl`, `NeanderthalClub.ctl`); `neanderthal_gigant.ep`, `neanderthal_gigant_kwongoarena.ep` and `neanderthal_noshield.ep` one club each; `elvian_gunner.ep` and `simba_gunner.ep` `NPC_Bazooka_R.ctl`; `zixie_m_plasma.ep` `NPC_Zapgun_R.ctl`; `simba_warrior.ep` and `zixie_gunner.ep` the identifier only. **No record has both** an identifier and a pointer; every named tool file exists under the `Tools` folder (all 68 `.ctl` files of the install are there). The vehicles' lists are empty (`vehicles.md`). The binary's name for the first member is read from its member table, which has the single name `mt_idToolObsolete` at offset 0 for the record (the second member is `mt_pctTool`); the identification with the data's `mt_idTool` is by position (**medium**; the data's own loader accepts the name `mt_idTool`, so the two names are one member).

### 2. Mounting (`CPuppetEntity::MountDefaultTools`, **medium**)

A virtual function of the puppet (the player puppet does not override it). For **each record of the parameter file's list** (`pp_amtMountedTools`) and then **each record of the entity's own list** (`en_amtMountedTools`), in order:
1. If the identifier is **not the invalid identifier**: it is looked up in the **tool-name table** (below). If a non-empty name is found, the resource of that name is **loaded** (as a character tool) and **stored in the record's pointer member, replacing any pointer**, and the identifier is **set to invalid**. (The record lives in the shared parameter object, so this is done once for all puppets that share it.)
2. The record's pointer is added to the puppet's **desired tools** (`AddDesiredTool(tool, 1)`, the second argument is a flag whose meaning belongs to T01-144). A null pointer is passed on as it is (the function's handling of null was not read).
After both lists the puppet updates its tools (`UpdateTools`), which attaches the desired tools (T01-144).

**The tool-name table.** A global list built **once when the module starts** by a file search for **`Tools/*.ctl`** below the database folder (the common path prefix of the weapon and parameter files): for each file found it stores the path and the **identifier made from the file's base name** (name without folder and extension). The lookup `identifier -> path` is a linear search (first match); a missing identifier gives the empty string and **the record is then left as it is** (identifier kept, pointer unchanged). So an identifier works only if a `.ctl` file of that base name exists in the folder.

**`CMountedTool::mdGetName`** (editor display name): the base name of the tool file of the **pointer** (empty without one), as an identifier turned to a string.

### 3. Throw descriptions in use (**medium**; the maths is `puppet-attacks.md` sections 8 and 9)

- **Record.** 24 bytes: the identifier, the pitch graph, the under-hit graph, the prediction graph (three pointers), the minimum and the maximum throwing speed (`CThrowDescription`; `mdGetName` is the identifier as a string). **Units** (`puppet-attacks.md`, read in `CalculateParams`): the pitch graph gives **degrees** (the code multiplies by π/180), the under-hit graph **metres** (pulled back from the target), the prediction graph a **ratio** (the share of the target's horizontal velocity that is led); all three are evaluated at the **distance** to the target; the speed window is in **metres per second** (3e38 = no upper limit).
- **`GetThrowDescription(identifier)`**: linear search over the puppet's list, **first record with an equal identifier**; none found: null (and the launch then uses the default of `puppet-attacks.md`: pitch 45 degrees, no under-hit, full lead, speed unbounded). In the data the identifiers within a file are distinct (script section 2).
- **Data (script section 2; high):** 30 descriptions in 15 files. `ThrowItem` (13 files; speed 10 to 30) and `ThrowFar` (13 files; speed 10 to 80) in all 13 NPC files (`chiche_female`, `chiche_male`, `elvian_f`, `elvian_f_old`, `elvian_m`, `elvian_m_old`, `simba_chief`, `simba_child`, `simba_female`, `simba_male`, `zixie_child`, `zixie_f`, `zixie_m`); the boss `kwongo.ep` has `CommonThrow` and the catapult `kwongocatapult.ep` `Throw` (speed 0 to 3e38); `elvian_m_old.ep` adds `Exact` and `zixie_m.ep` `ThrowExact` (the same window). The graph values are `puppet-attacks.md`.
- **The behaviour action `CBAThrowPickedTool`** (`batt_idThrowDescription`; `.cb` files `npc_civilian`, `npc_civilian_good`, `npc_civilian_static`, all with `ThrowItem`): its `Execute(brain)` finds the brain's puppet and
  - **with no identifier** (invalid): `ThrowPickedTool` and reports **success** (value 1): the tool taken from an entity (`GetToolAttachedFromEntity`: the first attached tool that has an entity, the picked object) is thrown with the **random launch** (no target);
  - **with an identifier**: `ThrowPickedToolAtTargetEntity(identifier, none)`: with no entity given the **puppet's current target entity** (a handle member of the puppet) is used; if there is none **nothing is thrown**; otherwise the same tool is thrown at it with that description. (The return value of this path was not read.)
  - In both paths nothing is thrown when the puppet holds no picked tool. The script functions `macThrowPickedTool` and `macThrowPickedToolAtTargetEntity(identifier, entity)` are the same two calls for level scripts (the entity argument defaults to none).
  So the **civilians' behaviour throws the object it picked at its target with `ThrowItem`**: pitch 45 degrees, 3 m short from 10 m on (the under-hit graph), no lead, speed window 10 to 30; `ThrowFar` (window 10 to 80) and the boss descriptions are **not named by any behaviour file** and are used only by scripts or animation events (not resolved: T01-226).
- **Picking.** `CBrainEntity::OnToolPicked` is the brain's notification after a tool was picked: it calls a virtual function of the brain with a fixed event identifier (the brain's behaviour system turns it into a behaviour event; the identifier text was not recovered): **T01-226**.

### 4. What the player does with tools (**medium**)

- The player brain (`CPlayerBrainEntity`, 76 exported symbols, `players-session-classes.md` section 4) has **no function that deals with tools or throw descriptions**; the player puppet has **no mounted tools** (its files have none) and no throw descriptions.
- **Rides.** The player's `StoreToolsBeforeRide` is **`PutDownWeapons(1)`** (both weapons are put down and their tools stored); `RestoreToolsAfterRide` **selects the same weapons again**: `SetNewWeapon(selected weapon, ...)` for the right hand and, when a left-hand weapon was selected (member of the puppet), the same for the left hand with the left-hand mode set for the call. Every other puppet's `StoreToolsBeforeRide` copies its **desired tools that have no entity** to the list `en_adctStoredToolsBeforeRiding` (after clearing it) and the restore (not read) is T01-144.
- **Weapon changes** treat the hand's weapon as a tool: `SetNewWeaponToPlayerHand` takes the tool of the hand's weapon (`GetWeaponTool(hand)`) and **stores** it (`StoreTool`) before the weapon object is put down or deleted (`weapon-pickups.md`, `weapon-view.md`). The tool files of the weapons are `bwp_pctLeftHandTool` and `bwp_pctRightHandTool` (`weapon-classes.md` section 4.1).

## Invariants

1. Every `CMountedTool` record has at most one of the two members set (10 of 10: 8 pointers, 2 identifiers); every named file exists (10 of 10).
2. A puppet has at most 2 mounted tools (1 record in 8 files, 2 in 1) and at most 3 throw descriptions (1 in 2 files, 2 in 11, 3 in 2); no file has two descriptions with the same identifier.
3. Every throw description has its minimum speed below its maximum (30 of 30); the maximum is 30, 80 or the "no limit" constant.
4. The tool-name table contains exactly the `.ctl` files of the `Tools` folders (68 in the install); an identifier without such a file is not resolved.
5. The three behaviour files that throw name `ThrowItem`, and all 13 NPC puppets have it.

## Open questions

1. **Who uses `ThrowFar`, `CommonThrow`, `Throw`, `Exact` and `ThrowExact`** (scripts, animation events of tools, the boss behaviours) and the event identifier of `OnToolPicked`: **T01-226.**
2. **The tool state machine and what `AddDesiredTool`, `UpdateTools`, `DrawTool`, `StoreTool` do:** now specified in [`puppet-tools.md`](puppet-tools.md) (T01-144; the throw maths is complete in `puppet-attacks.md`). A null tool is ignored by `AddDesiredTool` (so open question 3 is answered: harmless; **medium**).
3. **Whether a null pointer is harmless in `AddDesiredTool`** (no record of the shipped data has one after resolution, if the two identifier files are found): the handling was not read; with the run-time checks **T01-163**.

## Test vectors

1. **`simba_warrior.ep`** (identifier `NPC_Zapgun_R`, no pointer): at mount the table maps `npc_zapgun_r` to `.../Tools/NPC_Zapgun_R.ctl`, the file is loaded, the pointer is set and the identifier cleared; the tool is added as a desired tool. `zixie_m_plasma.ep` (pointer to the same file, empty identifier) gives the same tool without the lookup.
2. **A civilian** with a picked object and target at 12 m (`ThrowItem`: pitch 45 degrees, under-hit 3 m at 10 m or more, prediction 0, speed 10 to 30): `ThrowPickedToolAtTargetEntity(ThrowItem, none)` with the puppet's target: launch pitch from the vector to the target plus 45 degrees (capped at 81), aim point pulled 3 m back, speed clamped into 10 to 30 (`puppet-attacks.md` test vectors). With no target: nothing happens.
3. **Lookup:** a puppet with descriptions `[ThrowItem, ThrowFar]` asked for `ThrowFar` gets the second; asked for `Missing` gets none and the default parameters.
4. **Player ride:** mounting a vehicle puts both weapons down; leaving selects the remembered right-hand weapon and, if a left-hand weapon was selected, the left one.
