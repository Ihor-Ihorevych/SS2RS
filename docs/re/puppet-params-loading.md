# Where a puppet gets its parameter file: the two file tables, the generic branch and the `_DM` rule (T01-139)

- **Roadmap task:** T01-139 (follow-up of [`puppet-classes.md`](puppet-classes.md) section 3.2 and [`puppet-entity-classes.md`](puppet-entity-classes.md) section 2; the dummy foe's own loading is T01-150)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** only for what `tools/re/puppet_params_verify.py` reproduces from the data (the 124 files of the `Puppets` folder, the 104 of `EntityParams`, the 18 `_DM` files, the 2,328 placed puppets and their identifiers); **medium** for the loading rules read from `Sam2Game.dll` and `Core.dll` (each rests on **one reading** of the functions; the file-table builder was read in full); **medium-low** for the items marked so; **low** for the open questions. **Nothing was run.**

Prerequisites: [`puppet-classes.md`](puppet-classes.md) (3.2: the earlier outline, now refined), [`puppet-entity-classes.md`](puppet-entity-classes.md) (2: `OnCreate`), [`brain-assignment.md`](brain-assignment.md) (6: the placed pointers), [`game-modes.md`](game-modes.md) (the `_DM` files), `ep.md` (the `.ep` format).

## Summary

1. **A placed puppet never goes through the lookup in the shipped data.** All **2,458 placed property records** (2,328 puppets in 59 worlds) carry a pointer to their parameter file and none has the obsolete character identifier (`ep_idPuppet`: empty in all, `brain-assignment.md`). The lookup, the generic branch and the `_DM` rule below run only for a puppet whose placed pointer is **null**: the **player puppet** (created by code, not by a world) and puppets created by code or by an old file that still names a character.
2. **The order in `CPuppetEntity::OnCreate`** (section 1): the **obsolete identifier** is copied to `en_idPuppet`; each **instance mount** that still names a character gets its parameter file from the lookup; the **behavior** pointer is resolved from the obsolete behavior identifier when null; the **parameter pointer** is taken from the placed record, and **only when it is null** `GetPuppetParameters` is called; if the result is still null a **default `CPuppetParams` object** is built; finally **`en_idPuppet` := the name of the parameter resource** (its file name without folder and extension): this is why `en_idPuppet` equals the parameter file's stem in **2,328 of 2,328** placed puppets.
3. **Two file tables** are built **once when the module starts** (section 2): the **`Puppets` table** (every `*.ep` below `Content/SeriousSam2/Databases/Puppets/`, 124 files) and the **`EntityParams` table** (every `*.ep` below `.../EntityParams/`, 104 files). An entry is the **identifier of the file name without folder and extension** and the path; the lookup is a **linear search for the first entry** with an equal identifier; a missing identifier gives the empty string.
4. **Generic branch** (the six Character classes, whose virtual `IsGenericPuppet` answers true): the character identifier is looked up in the **`Puppets` table** and the file is accepted only when its class derives from the parameter class the entity type requires; **no deathmatch variant exists in this branch** (section 3).
5. **Non-generic branch** (every other puppet class, for example `CPlayerPuppetEntity`): the key is the **entity's class name with the leading `C` and the trailing `Entity` removed** (`PlayerPuppet`), looked up in the **`EntityParams` table**; **in a deathmatch game the first `.ep` of the path is replaced by `_DM.ep`** (`PlayerPuppet_DM.ep`); there is **no class check** and **no fallback to the base file** (section 4).
6. **A missing file** does not return null from the resource system: it returns a **resource proxy** that loads later (section 5). What a proxy for a file that does not exist resolves to was not established (open question 1).

## Evidence

- **Binary.** `Sam2Game.dll` and `Core.dll` of the install used for the other specs. Read in full (decompiled once each): `CPuppetEntity::GetPuppetParameters`, `GetPuppetParamsClass` (base and legged), the start-up builders of the two tables (their search folder, pattern, entry layout and identifier rule), the table lookup, the shared name-from-entity function and the deathmatch variant used by the weapon classes, `samIsDeathmatch`, `CPuppetEntity::OnCreate` (the parameter, behavior and mount parts), `samCreateGenericCharacter`, `CGenericLeggedPuppetEntity::OnCreate`, `CDummyFoePuppetEntity::OnCreate`, the vtable slots of the Character and generic-legged classes, and in `Core.dll` `resObtainResource` (with `mlScheduleResourceLoadByName`) and `strFileName`. Struct offsets were read by position and are not recorded.
- **Data.** `python -I tools/re/puppet_params_verify.py "<install>"` (about 12 minutes with the 59 worlds; `--no-worlds` about 20 seconds) prints sections 1 to 4 quoted below.
- **Not checked.** Nothing was run: no resource was actually missing, so the proxy case is read, not observed; the editor (not in the install) was not consulted; **who** creates a puppet without a placed pointer (other than the player) was not enumerated.

## Layout / rules

### 1. `OnCreate`: the order of the parameter steps (**medium**)

With the placed properties record `P` (a `CPuppetProperties`; with no record all the steps are skipped and the puppet gets the default object of step 6):
1. `en_idPuppet` := `P`'s **obsolete character identifier** (`ep_idPuppetObsolete`, the data's `ep_idPuppet`); the instance model, the multi-spawner flag, the patrol and flocking overrides, the default state, the barrier flag and the stretch are copied to the entity (`puppet-entity-classes.md` 9).
2. **Instance mounts** (`ep_hcpInstanceMounts`): for every mount of the **placed record** whose **character identifier is not invalid** the identifier is looked up in the **`Puppets` table**; when it gives a non-empty path, the file is loaded into the mount's parameter pointer (`cm_pppPuppetParams`) and the **identifier is set to invalid** (in the placed record itself). A mount with no table entry **keeps its identifier and gets no pointer**. Then the record is **copied** (an object copy with external handles copied) and the copy becomes `en_hcpInstanceMounts`.
3. **Behavior**: when `P`'s behavior pointer is null, the obsolete behavior identifier is looked up in a third table (every `*.cb` below `Databases/Behaviors/`, built the same way); a non-empty path is loaded into the pointer and the identifier is invalid afterwards. The entity's `en_pcbCharacterBehavior` := `P`'s pointer.
4. **Parameters**: when `P`'s `ep_pppPuppetParams` is **null**: `P`'s pointer := **`GetPuppetParameters(character identifier)`** (the identifier is the obsolete one copied in step 1) and `P`'s identifier := invalid. Then `en_pppPuppetParams` := `P`'s pointer. **When `P`'s pointer is set `GetPuppetParameters` is not called at all.**
5. A pointer that is still null here makes the entity build a **default `CPuppetParams` object** (constructor defaults of the class); a **proxy** pointer is **resolved** (the usual virtual that turns a handle object into the real resource).
6. `en_idPuppet` := the **name of the parameter resource** (`CEntityParams::GetName`: the file name without folder and extension, **medium**; 2,328 of 2,328 placed puppets agree with the stem of the file they point at, script section 3), and the three camera handles (`en_hcc3rdPersonRideCamera`, `en_hcc3rdPersonCamera`, `en_hcc3rdPersonDeathCamera`) are looked up in the file's camera table (`pp_cCameras`, `SCameras`).

### 2. The two file tables (**medium**; counts **high**)

Both are static tables created by start-up code of the DLL (once: the only caller of each builder that was found is that start-up code): the **search path** is the global database folder **`Content/SeriousSam2/Databases/`** followed by **`Puppets/`** or **`EntityParams/`**, the pattern is **`*.ep`**, the search is **recursive**, and for each path found an entry is appended: **(identifier of `strFileName(path)`, path)**. `strFileName` removes the folder and the extension (everything from the last dot), so the identifier of `.../Puppets/enemies/Kleer.ep` is `Kleer` (**medium**: the function was read; the data agrees, section 3 of the script). The lookup walks the entries **in the order of the file search** and returns the **path of the first entry whose identifier equals** the one asked for; none: the empty string. The search order is the file system's, so two files with the same stem in different folders would be resolved by **whichever the search listed first**; in the install there is **no such pair**: the 124 stems of the `Puppets` folder are distinct (also ignoring case).

Data (script sections 1 and 2): the `Puppets` folder has **124** files in 12 sub-folders (`enemies` 48, `npcs` 22, `vehicles` 21, `turrets` 11, `bosses` 7, `animals` 4, `npcs_cutsequences` 4, `bossturrets` 3, `bossvehicles` 1, `enemies_cutsequences` 1, `objects` 1, `player` 1); `objects/` holds only `cactus.ep` (**no `DummyFoe.ep`**, T01-150); `player/` holds `player.ep`. The `EntityParams` folder has **104** files: the weapon parameter and rendering files, the projectile and item files and **`PlayerPuppet.ep` with `PlayerPuppet_DM.ep`**; **18** `_DM` files exist and each has its base file (17 weapon files and the player's).

### 3. The generic branch (`IsGenericPuppet` true; **medium**)

The virtual `IsGenericPuppet` (slot of the puppet class; the base answers **false**) is overridden to **true** by the six Character classes (`CLeggedCharacterEntity`, `CAircraftCharacterEntity`, `CHovercraftCharacterEntity`, `CRollingBallCharacterEntity`, `CTurretCharacterEntity`, `CWheeledVehicleCharacterEntity`; **medium**: by the vtable position and the shared "return 1" function) and **not** by `CGenericLeggedPuppetEntity` (whose parameters are embedded in its properties record, below). `GetPuppetParameters(identifier)`:
1. the required parameter class's identifier comes from the virtual **`GetPuppetParamsClass`** (the base answers invalid; legged `CLeggedPuppetParams`, aircraft `CAircraftPuppetParams`, hovercraft `CHovercraftPuppetParams`, rolling ball `CRollingBallPuppetParams`, turret `CTurretPuppetParams`, wheeled vehicle `CWheeledVehiclePuppetParams`); an **invalid class identifier, or a class the type system does not know**, returns **null** silently;
2. the identifier is looked up in the **`Puppets` table**; an **empty path** returns **null** silently;
3. the path is loaded as a `CPuppetParams` (section 5); a null result returns null silently;
4. the loaded object's class must **derive from the required class**; otherwise the console prints **`Parameters for character "<identifier>" are not of proper class ("<required class>")`** and the result is **null**.
**There is no deathmatch test in this branch** (checked in the code: the only `_DM` rule of the function is in the other branch), so a generic character is the same in every game mode, and the install has no `_DM` file under `Puppets`.

`CGenericLeggedPuppetEntity` (not in any shipped world) copies `en_pppPuppetParams` from its properties; when that pointer is null it takes a **copy of the parameter object embedded in the properties** if that object is of the generic legged parameter class, and when a model path string is set in it, it loads that model and clears the string.

### 4. The non-generic branch and the `_DM` rule (**medium**)

For a puppet whose `IsGenericPuppet` is false the identifier argument is ignored:
1. the key is **the entity's data-type name with the leading `C` and the trailing `Entity` removed**, converted to an identifier (`CPlayerPuppetEntity` gives `PlayerPuppet`; a type without a name gives the empty identifier);
2. it is looked up in the **`EntityParams` table**; an empty path returns **null** silently (no warning);
3. **if the game is a deathmatch** (`samIsDeathmatch`: the game info entity's game type **equals 4, `GT_DEATHMATCH`**; team deathmatch 8 and capture the flag 16 are **different values**, so by this test they use the base files: **medium-low**, because whether the stored game type can hold combined flags was not checked) **the substring `.ep` of the path is replaced by `_DM.ep`**;
4. the path is loaded as a `CPuppetParams`; **no class check** is made and there is **no fallback** to the base file when the `_DM` file is missing (the resource then follows section 5).
In the data this branch is used by **one class**: `CPlayerPuppetEntity` with `EntityParams/PlayerPuppet.ep` and `PlayerPuppet_DM.ep` (the differences are `game-modes.md`). The **weapon classes** use the same two-step rule in a shared function (a name from the entity class, a table lookup, the same `.ep` to `_DM.ep` replacement) and so do the **item and projectile** classes without the deathmatch step in their own create functions; the 17 weapon `_DM` files are the same convention (`weapons.md`).

### 5. Loading and missing files (**medium**)

The loads use `resObtainResource` with the class of `CPuppetParams`, the first flag 0 and the other two flags 1: it first looks for the **file in the resource stock**; if the file is not there it **schedules a load by name** and returns a **resource proxy** (a handle object that stands for the resource until the load ends), **never null**. A caller that holds a proxy resolves it through the resource's virtual when it reads the pointer (the `OnCreate` code does this after step 5 above, and the same "if the flag bit is set, ask the object for its replacement" pattern is used by every member that holds a resource pointer). So for a path that names a file that **does not exist**, `GetPuppetParameters` returns a **non-null proxy**, `OnCreate` does **not** build the default object (the null test comes before the proxy is resolved) and what the proxy resolves to is **not established** (open question 1). The `Parameters ... not of proper class` warning is the only message of this function; the resource system's own messages for a failed load were not read.

### 6. Where `en_idPuppet` is used (**medium**, cross-references)

`CanBecomeMyEnemy` refuses two puppets with the same non-empty `en_idPuppet` (`brain-senses.md` 5), so **the file stem is also the "same species" test**: puppets of one parameter file never become each other's enemies. The brain's `GetAvailableSeat`, `FindBestSeat` and the instance mounts do not use it.

## Invariants

1. The 124 identifiers of the `Puppets` table are distinct; no file of the `Puppets` folder has a `_DM` sibling; every `_DM` file of `EntityParams` has a base file (script, **high**).
2. A placed puppet whose record has a parameter pointer never calls `GetPuppetParameters` (**medium**, one function); all 2,458 placed property records have one (script, **high**).
3. For all 2,328 placed puppets `en_idPuppet` equals the stem of the pointed parameter file (script, **high**), which is the rule of step 6 of section 1.
4. The `_DM` replacement happens only in the non-generic branch and only when the game type is 4 (**medium**).

## Open questions

1. **What a proxy for a missing file resolves to** (a null pointer, a default object, or an error and a stop), and the resource system's own message: **T01-323** (with the run-time checks of the loader).
2. **Who creates a puppet with no placed pointer** besides the player (script and code paths that name a character identifier, `samCreateGenericCharacter` callers with an identifier), and the exact game-type values for team deathmatch and capture the flag: **T01-323** and **T01-130**.
3. **The behavior table** (`Databases/Behaviors/*.cb`) was only seen in its builder; its identifier rule and the second lookup function were not read: **T01-323**.

## Test vectors

1. **Player puppet, single player** (a puppet of class `CPlayerPuppetEntity` with no placed pointer, game type 1): key `PlayerPuppet` → `.../EntityParams/PlayerPuppet.ep`; loaded; `en_idPuppet` := `PlayerPuppet`. **Deathmatch** (game type 4): the path becomes `.../EntityParams/PlayerPuppet_DM.ep`; `en_idPuppet` := `PlayerPuppet_DM`. **Capture the flag** (game type 16): `PlayerPuppet.ep` (by the equality test).
2. **A generic legged character with the obsolete identifier `Kleer`**: required class `CLeggedPuppetParams`; the `Puppets` table maps `Kleer` to the first file of that stem; the file is loaded; if the loaded file is of another class (for example an aircraft file), the warning `Parameters for character "Kleer" are not of proper class ("CLeggedPuppetParams")` and a null result (then the default object).
3. **An identifier with no table entry**: the empty path; null; `OnCreate` builds the **default** `CPuppetParams`; `en_idPuppet` := the name of that object (not established, open question 1).
4. **A placed puppet** with a pointer to `enemies/Kleer.ep`: `GetPuppetParameters` is not called; `en_idPuppet` := `Kleer`.
5. **An instance mount** with character identifier `Kleer` and no pointer: the pointer is set from the table and the identifier is cleared (the only mount in the shipped data has an empty identifier and a pointer: `puppet-carry-rides.md` 4).
