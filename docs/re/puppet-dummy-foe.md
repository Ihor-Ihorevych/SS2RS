# The dummy foe puppet: a code-only test target that nothing in the install uses (T01-150)

- **Roadmap task:** T01-150 (follow-up of [`puppet-vehicle-classes.md`](puppet-vehicle-classes.md) open question 4 and [`brain-assignment.md`](brain-assignment.md) section 8)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/dummy_foe_verify.py` and `tools/re/puppet_params_verify.py` reproduce from the install (no file, no placed entity and no script names the class; the one palette entry); **medium** for the rules read from the 14 overridden functions of `CDummyFoePuppetEntity` in `Sam2Game.dll` (each decompiled once, in full); **medium-low** for the item marked so; **low** for the open questions. **Nothing was run.**

Prerequisites: [`puppet-vehicle-classes.md`](puppet-vehicle-classes.md) (the class table; this page replaces its name-level rows for the dummy foe), [`puppet-params-loading.md`](puppet-params-loading.md) (how parameter files are found), [`brain-assignment.md`](brain-assignment.md) (8: its brain), [`puppet-entity-classes.md`](puppet-entity-classes.md).

## Summary

1. **What it is.** A **target dummy**: a puppet that can be placed, has a fixed model, a speed it follows when given a goal point and a health that is either "never dies" or a number, and **no behavior and no foe logic**. The entity class is `CDummyFoePuppetEntity` (a direct child of `CPuppetEntity`, 14 overridden functions), its properties `CDummyFoePuppetProperties` (4 own members), its parameter class `CDummyFoePuppetParams` (**no members**) and its brain `CDummyFoeBrainEntity` (no foe manager).
2. **Who creates it: nobody that the install shows.** (a) **No world** places one (0 entities of the class in all 59 worlds, `puppet_params_verify.py` section 4). (b) **No code path creates one**: the only function of the DLL that creates a character from a class name (`samCreateGenericCharacter`) has **one caller, `MountRiders`**, which builds the class name from a *puppet parameter* file's class (`...PuppetParams` becomes `...CharacterProperties`), and that never yields the dummy foe's class (its params class is `CDummyFoePuppetParams`, whose name would give `CDummyFoeCharacterProperties`, which does not exist); the class name occurs in the DLL only in its own functions and in the start-up registration. (c) **No data names it**: of the **8,063 data files** the archive reader exposes exactly **one** has the text `DummyFoe` in its bytes: the editor's entity palette (`Content/SeriousEditor2/CustomTrees/entities.ctr`), where it is **one of the 110 entries of the game module**, next to `Legged character`, `Aircraft character`, `Hovercraft character`, `Rolling ball character`, `Turret character`, `Wheeled vehicle character`, `Generic legged puppet`, `Test body` and `Test buggy`. No `.cb`, `.rsc`, `.ep`, `.wld`, `.mdl`, macro text or `.nfo` of the install mentions it, and none of the five other game binaries searched (`Sam2.exe`, `Engine.dll`, `Sam2GameGui.dll`, `EngineGui.dll`, `DedicatedServer.exe`) has a string with that name.
3. **So it is an editor-only test entity.** A designer can drop one from the palette (the registration also names an editor thumbnail `Content/SeriousEditor2/Thumbnails/DummyFoe.tga`, which is **not** in the install), set its alignment, flying flag, velocity and health, and use it as a **moving or standing target for testing weapons and scripted goal points**. This is an inference from the palette entry, the missing behavior and the fixed model (**medium-low**); the editor itself is not in the install.
4. **Its built-in files are absent.** The class loads **`Puppets/Objects/DummyFoe.ep`** (relative to `Content/SeriousSam2/Databases/`) and the model **`Content/SeriousSam2/Models/Enemies/DummyFoe/DummyFoe.mdl`**; **neither exists in the install** (the `Puppets/Objects` folder has only `cactus.ep`, there is no folder or file with `dummy` in its name). Their contents therefore **cannot be specified**; what the class itself fixes is in sections 2 to 4.
5. **A quirk (medium-low).** `OnCreate` loads `DummyFoe.ep` into the **placed properties record** **after** the base class has already created the entity's parameter object; so on the **first** creation the entity runs on the **default `CPuppetParams`** (or whatever the base lookup yielded) and the file only reaches a record that is **created again** from the same properties (section 3).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Read in full (decompiled once each): `CDummyFoePuppetEntity::OnCreate`, `OnBoot`, `OnReceiveDamage`, `GetVelocity`, `GetDeclaredNominalVelocity`, `MovingIn3DArea`, `LerpToDesiredVelocityAndLook`, `DoDeathSequence`, `CallShutDown`, `SetCurrentMechanism`, `GetPuppetModel`, the constructors of the entity, its properties and its parameter class, `CDummyFoeBrainEntity`'s override, `CPuppetEntity::OnCreate` (the parameter, behavior and alignment steps), `GetPuppetParameters`, `samCreateGenericCharacter` and its callers, `SetHealth`, and the references to the class name and the two paths over the whole DLL. The member names (`ep_baAligment`, `ep_bFlying`, `ep_fVelocity`, `ep_slHealth`; `en_bImmortal`, `en_bFlying`, `en_fVelocity`, `en_slCustomHealth`) are those of `puppet-vehicle-classes.md` (the declared tables; by position here); `Sam2.exe`, `Engine.dll`, the two GUI DLLs and the dedicated server were searched for strings containing `dummy` (none relevant: the engine's own dummy hull and dummy GFX/SFX wrappers).
- **Data.** `python -I tools/re/dummy_foe_verify.py "<install>" --grep-all` (about 70 s) prints the file-name and palette results quoted above; `python -I tools/re/puppet_params_verify.py "<install>"` (section 4, with the worlds, about 12 minutes) the world count.
- **Not checked.** Nothing was run; **the editor was not available**, so how the palette entry creates the entity (which properties it starts with) is inferred from the class's constructor defaults; the model and parameter file are absent, so nothing about them beyond the paths could be read; whether the original developers' own tools (outside the install) used the class cannot be answered from the install.

## Layout / rules

### 1. Classes and members (**high** for the declared names, **medium** for meanings)

| Class | Parent | Members | Defaults (constructor) |
|---|---|---|---|
| `CDummyFoePuppetProperties` (data `SDummyFoePuppetProperties`) | `CPuppetProperties` | `ep_baAligment` (the spelling is the binary's), `ep_bFlying`, `ep_fVelocity`, `ep_slHealth`, in this order | alignment **0** (`BA_GOOD`), flying **false**, velocity **10.0**, health **-1** |
| `CDummyFoePuppetEntity` (data `SDummyFoePuppetEntity`) | `CPuppetEntity` | `en_bImmortal`, `en_bFlying`, `en_fVelocity`, `en_slCustomHealth`, in this order | all **0** |
| `CDummyFoePuppetParams` | `CPuppetParams` | none | the base defaults |
| `CDummyFoeBrainProperties` / `CDummyFoeBrainEntity` | `CBrainProperties` / `CBrainEntity` | none | answers **no** to `HasFoeManager` (`brain-assignment.md` 8) |

`en_bImmortal` is **never written and never read** by any function read: the immortality test uses the base health instead (section 4). **The class is in no data file** (`SDummyFoePuppetEntity` has 0 stored records), so the stored layout is not known from data.

### 2. Creation (`OnCreate`; **medium**)

In order: (1) the properties record is cast to the dummy foe's class (anything else gives no record and the next step is skipped); (2) **`en_baAlignment` := `ep_baAligment`, `en_bFlying` := `ep_bFlying`, `en_fVelocity` := `ep_fVelocity`, `en_slCustomHealth` := `ep_slHealth`**; (3) the base `OnCreate` runs (`puppet-params-loading.md` 1: parameters, the default object, **the alignment is overwritten from the behavior only when a behavior exists**: a dummy foe has none, so the property value stays); (4) when the body (mechanism) exists and **`en_bFlying` is not 0**: the body's flag bit that the base `OnStep` also sets when the move-ability bit 2 appears (**medium-low**: its name was not resolved) is set and **`en_maMoveAbilities` gets bit 2** (`MA_...`, the value `OnBoot` and `OnStep` also test); (5) **when the placed record has no parameter pointer** the record's pointer is set to the file **`Puppets/Objects/DummyFoe.ep`** (loaded with the resource system, class `CPuppetParams`) **after** step 3.

Consequence (**medium-low**): in step 3 the base class already took the record's null pointer, looked the puppet up (non-generic branch, key `DummyFoePuppet`, no such file in the `EntityParams` table: `puppet-params-loading.md` 4: empty path, null) and **built a default parameter object for the entity**; step 5 then fills the **record**, not the entity. A **second creation from the same record** (for example a level reset that reuses the properties object) finds the pointer set and uses `DummyFoe.ep`. Whether the shipped editor created entities this way twice, or the file simply was meant to be found by the first creation, cannot be told from the install.

`GetPuppetModel` returns the fixed model **`Content/SeriousSam2/Models/Enemies/DummyFoe/DummyFoe.mdl`** (the generic mechanism of the base class takes it from there; `GetPuppetParamsClass` is the base's invalid answer, so no class check applies).

### 3. Boot, health and death (**medium**)

- **`OnBoot`**: the base boot, then **`en_maMoveAbilities |= 2` when `en_bFlying`** (again), then **when the puppet's health `en_slHealth` is above 0** `SetHealth(en_slCustomHealth)` and `SetHealthMax(en_slCustomHealth)`: so the base health that comes from the (missing) parameter file is replaced by the property's **custom health** (with the constructor default **-1** both become **-1**).
- **`OnReceiveDamage`**: **returns at once when `en_slHealth` is below 0**, else the base damage processing (`puppet-entity-classes.md` 7). So **immortal means a negative health** (the default -1), not `en_bImmortal`; a positive custom health makes a killable target with exactly that health and maximum.
- **`DoDeathSequence`**: calls `CallShutDown` immediately: a dying dummy foe plays **no death animation** and is shut down in the same call (`OnShutDown`: `puppet-entity-classes.md` 8), after the base `DoDying` that raised the event.

### 4. Movement (**medium**; one number medium-low)

The puppet is moved **kinematically** on goal-point orders and ignores the base movement models:
- **`SetCurrentMechanism`**: the base switch, then, when the puppet's joint is a **linear-motion joint**: freedom axes **forward `(0, 0, -1)` and up `(0, 1, 0)`** (the engine's forward is -z), **acceleration force 1500**, **break force 1500**, **velocity 0** (units of the physics engine, not converted).
- **`GetDeclaredNominalVelocity`** = `en_fVelocity`. **`GetVelocity`**: **zero** when the move type is `MT_NONE` (0); otherwise the **unit vector of `en_vDesiredTempoAbs` times `en_fVelocity` times `en_fSpeedMultiplier`** (zero when the desired tempo is zero).
- **`MovingIn3DArea`** = `en_bFlying`.
- **`LerpToDesiredVelocityAndLook`** acts **only for the move type `MT_GOAL_POINT` (1)**: with `v` = the length of `GetVelocity()` and `dt` = the simulation step in seconds: `d` = the goal point minus the position; **if `|d| ≤ dt × v` the puppet is placed exactly on the goal point** (orientation unchanged); otherwise it moves **`dt × v` along `d`** and its orientation becomes a **heading only**: `heading = atan2(-dx, -dz)` of the unit direction (so heading 0 faces -z), **0 when the unit direction's y is above 0.99 or below -0.99**; the pitch and bank are written as 0 in the code read (**medium-low**: the Euler layout of the temporary was read by position). The result is set with `SetAbsPlacement`. **This function writes no "goal point reached" flag** (whether anything else sets it was not checked), and **the other move types (`MT_DESIRED_TEMPO_LOOK`, `MT_DRIVE_STEER_LOOK`) do not move the dummy foe at all**.
- **Speeds:** `en_fVelocity` default **10.0** (property default); the speed multiplier is the base member.

### 5. What a designer saw (**low**, inference)

Placed from the palette with the constructor defaults, a dummy foe is a **good-aligned, immortal (health -1), non-flying, 10-unit-per-second** target whose model and parameter file the shipped game does not contain, so **in the shipped build it could not have worked as a visible target**: the model path does not resolve, and the entity's parameters are the base default object. Everything else in the install treats it as absent: no brain behavior, no world, no macro.

## Invariants

1. No file of the install except the editor palette contains the text `DummyFoe` (8,063 files, script, **high**); no world places the class (59 worlds, **high**).
2. A dummy foe never runs a behavior (`HasFoeManager` answers no; no `.cb` can be given by the data since no record exists) and its damage is ignored while `en_slHealth < 0` (**medium**).
3. `LerpToDesiredVelocityAndLook` moves the puppet only for `MT_GOAL_POINT` and never farther than `dt × |GetVelocity|` per step (**medium**).

## Open questions

1. **The contents of `Puppets/Objects/DummyFoe.ep` and `DummyFoe.mdl`** (a model name, a health, a hull, an animation set): not in the install; **cannot be answered from it** (a run-time check is impossible too: the game does not load them). Closed as "not obtainable" unless a build that has them appears (**T01-130** would only confirm the absence).
2. **Whether the editor starts a palette entity with the constructor defaults** and whether the properties-record quirk of section 2 (the file loaded into the record after the entity's parameters) is exercised in the editor's own run mode: needs the editor (not in the install): **T01-324**.

## Test vectors

1. **Defaults, goal point 5 units away, `dt` = 0.05 s (20 steps per second), `MT_GOAL_POINT`:** `v = 10 × 1 = 10`, `dt × v = 0.5` per step: the puppet reaches the goal point after **10 steps** (the 10th step has `|d| = 0.5 ≤ 0.5` and snaps onto it); the heading is `atan2(-dx, -dz)` of the unit direction while moving.
2. **Damage:** default properties (health -1): `OnBoot` sets health and maximum to **-1**; any damage is ignored (**immortal**). With `ep_slHealth` 200 and a base health above 0: health and maximum become **200**; damage works as for any puppet and on death `DoDeathSequence` shuts the puppet down at once.
3. **Flying:** `ep_bFlying` 1: `en_maMoveAbilities` has bit 2 from `OnCreate` and `OnBoot`, `MovingIn3DArea` answers yes.
4. **No behavior:** the alignment is the property's (default 0, `BA_GOOD`); with a behavior it would be the behavior's.
