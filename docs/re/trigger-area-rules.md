# Detector areas, killers, environment damagers and alarms: rules (T01-169)

- **Roadmap task:** T01-169 (follow-up of `mechanism-classes-triggers.md` open question 1). Classes, members and counts: [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md). Brains and threat sounds: `brain-senses.md`.
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the `EDamageType` values (the binary's enumeration table, and the killer's two constants agree with it), for the freezing numbers (read from the parameter files) and for the data tables (all 59 worlds, script `tools/re/trigger_area_verify.py`); **medium** for every rule below (one reading of the code each, lost arguments recovered from the instruction listing); **low** for the points marked "open". **Nothing was run.**

Prerequisites: [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md), `level-logic.md` and `level-logic-api.md` (the events and macro commands), `brain-senses.md` section 5.1, `main-loop.md` (variable simulation steps).

## Summary

1. **The killer's "second damage call" is the instant kill, and it is not an extra call: it is the other branch.** With `ep_bKillInstantly` 0 **and** `ep_fDamagingPeriod` above 0 the killer deals `ep_slDamageAmount` of damage type **24 (`DMT_KILLER`)** to **any** entity at most once per period (the first time one period after the first touch). In **every other case** (kill instantly, or a period of 0 or less) it deals **lethal damage to puppets only**: an amount of **2,147,483,647** of type **24**, or **25 (`DMT_ABYSS`) when `ep_bAbyss` is 1** (section 3). The earlier reading of `mechanism-classes-triggers.md` (two calls per touch) was wrong.
2. **A detector's target is notified through the target's `OnTrigger(entity)`**: the chapter info (a chapter change flow, T01-172) and the animated mover (start moving) implement it; the call happens at every touch of a valid entity in normal mode (section 2).
3. **A detector in normal mode is one-shot**: the first touch removes its hull, turns the editor colour to the active colour and raises `Activated`; `Recharge` re-arms it. **In custom mode** the hull collides with nothing until scripts register entities; a registered entity touching it raises `CustomDetection` and is removed from the list (section 2).
4. **Environment damagers are freezing zones for players**: only `CPlayerPuppetEntity` reads them. A player standing in a non-healing zone builds a **freezing ratio** from 0 to 1 over `ppp_fFreezeTime` (**30 s** or **60 s**, by parameter file), loses it at 1 per `ppp_fUnfrezeTime` (**5 s**) outside, and at ratio 1 takes **1 point of `DMT_COLD` (15) at 1.5 per second**; **a healing zone always wins** (section 4).
5. **Alarms are not used in the retail game**: none is placed in the 59 worlds and no script calls the only macro function; an alarm is an editor entity that, when a script calls `EmanateThreatSound`, alerts every foe-less brain closer than its heard distance (section 5).

## Evidence

- **Code.** `CDetectorAreaEntity::OnHullTouched`, `Recharge`, `Deactivate`, `RegisterCustomDetection`, `UnregisterCustomDetection`, `SetupCustomDetectionCollisionCategory`, `CreateHull`, `OnCreate`, `OnBoot`; `CKillerEntity::OnHullTouched` and `GetEntityAndDamageTime`; `CPuppetEntity::ApplyLethalDamage`; `CPlayerPuppetEntity::ApplyEnvironmentDamage` and its caller; `CWorldInfoEntity::CollectEnvironmentDamagers`; `CAlarmEntity::EmanateThreatSound` and `OnCreate`; `samEmanateThreatSound`; `CChapterInfoEntity::OnTrigger`; the entity-class vtables (the "trigger" slot). The `EDamageType` enumeration table was read as data; lost arguments were read in the instruction listing. Build: the retail DLLs of the install used by all T01 pages.
- **Data.** `python -I tools/re/trigger_area_verify.py --jobs 4` (about 25 minutes): detectors, killers, locators, the environment damagers' boxes and the freezing numbers of the player parameter files. Counts below are from that run.
- **Not checked.** What the damage pipeline does with types 24 and 25 (invulnerability, armour, god mode), the target's `OnTrigger` for props, any timing on the running game.

## Layout / rules

### 1. `EDamageType` (high)

The binary's enumeration table (value, name): **-1 `DMT_NONE`**, 1 `DMT_PIERCING`, 2 `DMT_CUT`, 3 `DMT_CUTTING`, 4 `DMT_PUNCH`, 5 `DMT_BLUNT`, 6 `DMT_CRUSHING`, 7 `DMT_KICKING`, 8 `DMT_IMPACT`, 9 `DMT_FALLING`, 11 `DMT_EXPLOSION`, 12 `DMT_SHOCKWAVE`, 13 `DMT_DROWNING`, 14 `DMT_HEAT`, **15 `DMT_COLD`**, 16 `DMT_ELECTRICITY`, 17 `DMT_ACID`, 18 `DMT_PLASMA`, 19 `DMT_BULLET`, 20 `DMT_SNIPER_BULLET`, 21 `DMT_SLUG`, 22 `DMT_KILL_ALL_CHEAT`, 23 `DMT_SERIOUS_BOMB`, **24 `DMT_KILLER`**, **25 `DMT_ABYSS`** (25 values; 0 and 10 are not in the table). The killer deals 24 or 25; the freezing zone deals 15. Two places that depend on the type were found: the puppet's hit-reaction function (`ApplyDamageImpact`) is **skipped for types 24, 9, 14 and 15** (killer, falling, heat, cold); the death camera tests a puppet's stored death type for **25** (`RendersIn3rdPerson` and `Get3rdPersonCameraPlacement`: the effect on the view of a player killed by an abyss is **low**, open).

### 2. Detector areas (read; medium)

**Members used (binary names).** `en_hhu` (the hull), the target entity (`ep_hepTarget`: the target record's spawned entity; 3 of 1,077 detectors have one), `en_bActivated` (the "activated" flag), `en_henLastActivator`, `ep_bCustomEventDetectorArea` ("custom"), `ep_bPlayersOnly`, `ep_colActive` and `ep_colInactive`, `en_eaCustomDetections` (the registered entities).

**`CreateHull`.** A primitive hull of the record's shape is made at the detector's placement and attached to the body. Collision category: **custom mode**: `SetupCustomDetectionCollisionCategory`; **normal mode**: `playerdetector` when `ep_bPlayersOnly` else `detector`.

**`OnHullTouched(hull)` (host).** Proceeds only when the touching hull has an **owner entity** (not world geometry). Then:

1. `en_henLastActivator` := the owner (**also** for a touch by a non-registered entity in custom mode, if one arrives).
2. **Normal mode** (`ep_bCustomEventDetectorArea` 0):
   - if there is a **target entity** and the owner exists: **`target.OnTrigger(owner)`** (a virtual of the entity base class, "trigger"; implemented by `CChapterInfoEntity` and `CAnimatedMoverEntity`; the base and the prop do nothing that was found) **at every touch**, before the flag test;
   - if the flag `en_bActivated` is 0: delete the hull (no further touches), set the editor colour to the **active** colour, flag := 1;
   - raise the macro event **`Activated`** (also on a repeated touch that was already queued).
3. **Custom mode** (97 detectors): if the owner is **in `en_eaCustomDetections`**: raise **`CustomDetection`**, remove the owner from the list, re-run `SetupCustomDetectionCollisionCategory`. A touch by an entity not in the list does nothing but step 1.

**`SetupCustomDetectionCollisionCategory`.** Null entries are cleared from the list; the hull's category := **`custom_detector`** when the list is not empty, **`ignoreall`** when it is empty (nothing collides with it). So a custom detector is **inert until a script registers entities** and becomes inert again when the last registered entity has been detected.

**Script commands.** `RegisterCustomDetection(entities)` (121 + 5 uses): converts the argument (one entity or a container) to entities and adds each non-null one that is not yet in the list, then sets the category; `UnregisterCustomDetection` (1 use) removes them; **both log an error and do nothing when the detector is not in custom mode** ("Macro function ... is called on detector area that is not marked as \"Custom entity detector\""). `GetLastActivator` returns `en_henLastActivator` (16 uses). `Recharge` (147 + 136 uses): flag := 0; **create the hull if there is none** (a hull deleted by an earlier activation is rebuilt); clear the renderable's "hidden" bit; colour := inactive. `Deactivate` (1 use): clear the custom list, delete the hull, flag := 1 (the detector stays dead until `Recharge`). `OnBoot` (new level or chapter restore): last activator := none, then `Recharge`.

**Consequences.** A normal detector raises `Activated` once per `Recharge`; handlers that want a repeatable trigger call `Recharge` on it (`level-logic.md`). A detector whose hull is deleted still exists: scripts can recharge it any time. The target notification (`OnTrigger`) is **not** limited to the first touch: it is called at every touch event that reaches the handler before the hull is removed, which for a one-step overlap is typically one call.

### 3. Killers (read; medium)

`CKillerEntity::OnHullTouched(hull, ..., hit point, normal)`, for the owner entity of the touching hull (a `CBaseEntity`; none: nothing):

**A. Periodic damage** (only when `ep_bKillInstantly` is **0** and `ep_fDamagingPeriod` is **above 0**; any kind of entity):

1. Find the entry of the owner in the killer's list of (entity, time of last damage); if there is none, **append one with the time := now**.
2. If `now - time < ep_fDamagingPeriod`: return (**the first damage comes one period after the first touch**, as long as touches keep arriving).
3. Otherwise build a damage record: amount **`ep_slDamageAmount`**, type **24 (`DMT_KILLER`)**, inflictor = this killer, the hit point and normal of the touch, the hull; call the owner's **`InflictDamage`** (the standard damage entry of every entity); the entry's time := now.
4. Remove the list entries whose entity no longer exists.

**B. Lethal damage** (every other case: `ep_bKillInstantly` 1, or a period of 0 or less): only when the owner is a **puppet**: `ApplyLethalDamage(direction zero, type, no weapon)` with type **25 (`DMT_ABYSS`) if `ep_bAbyss`, else 24**. `ApplyLethalDamage` builds a damage record with the amount **2,147,483,647**, that type, the puppet itself as inflictor (when no source is given), the puppet's position as the hit point, no hull, and calls **`InflictDamage`** (the same entry as A: **invulnerability, shields and the like are applied by that entry**, not by the killer; what they do with types 24 and 25 is open). **No time list is kept**: the call is repeated at every touch event. Non-puppet entities are not affected by a kill-instantly killer.

**Data (script section 2; all 104 killers).** **Lethal branch 89** (kill instantly 1): **76 with type 24 and 13 with type 25** (the 13 abyss killers all have amount 10 and period 1, kill instantly 1). **Periodic branch 15** (kill instantly 0, every one with a period above 0): period 1 s with amounts 40 (4), 100 (2), 50 (1), 1 (1); period 0.1 s with amounts 5 (3) and 20 (1); period 0.5 s with amounts 50, 20 and 5 (one each). `ep_slDamageAmount` and `ep_fDamagingPeriod` are irrelevant for the 89 lethal killers (amounts 10 in 87, 100,000 and 1,000 once each: stored but unused; periods 1, 100 and 0.0001). 29 killers are children of an animated mover and move with it, 75 are free (`mechanism-classes-triggers.md`).

### 4. Environment damagers: the freezing zones (read; medium)

**Who reads them.** `CWorldInfoEntity::CollectEnvironmentDamagers` (at simulation start and after a world change) gathers all `CEnvironmentDamagerEntity` of the world into the world info's list; **only `CPlayerPuppetEntity::ApplyEnvironmentDamage`**, called from the player's per-step update, reads the list. Enemies and other entities ignore the zones. The damager has no behaviour of its own (`ep_edtType` is not read: 0 in all 23).

**The test.** The player's **centre point** (the centre of its bounding box moved by its placement) is tested against every damager's **oriented box** (the box `ep_boxEnvironment`, relative to the damager's placement, `mthOBBoxTouchesPoint`). Let *inside* be "the point is in at least one box" and *protected* "in at least one box with `ep_bHealing` 1"; the scan stops at the first protected box, so **protection overrides any number of damaging boxes**.

**The freezing ratio** `en_fFreezingRatio` (0 to 1, per player) each step, with *dt* the simulation step and the player parameters `ppp_fFreezeTime` *Tf*, `ppp_fUnfrezeTime` *Tu*, `ppp_fFreezingDamagePerSec` *R*, `ppp_fFreezingExhalePeriod`:

1. **inside and not protected**: if the ratio is below 1 and *Tf* is not 0: ratio := ratio + *dt* / *Tf*;
2. **otherwise** (outside every box, or protected): if *Tu* is not 0: ratio := ratio - *dt* / *Tu*;
3. if the ratio exceeds 1 before clamping, the time of the last freezing damage := now; clamp the ratio to [0, 1];
4. **if the ratio is exactly 1**: period := 1 / *R* (1 s when *R* is 0); if `now - last damage time > period`: last damage time += period; inflict **1 point of `DMT_COLD` (15)** on the player (the damage entry of section 3).

So a player must stay **Tf seconds** in a damaging zone to freeze completely, thaws in **Tu seconds**, and then loses **R points per second** (steady rate, one point every 1/*R* s, catching up the schedule rather than resetting it). **Numbers (script section 5; high):** `ppp_fFreezeTime` **30 s in `player.ep` and `santa.ep`, 60 s in `playerpuppet.ep` and `playerpuppet_dm.ep`**; `ppp_fUnfrezeTime` **5 s**; `ppp_fFreezingDamagePerSec` **1.5**; `ppp_fFreezingExhalePeriod` **3 s** (the breath puffs of `UpdateFreezingExhalation`, not read). Which file applies to which player (the 30 s and 60 s files) is open (**T01-309**). The ratio also drives the freezing HUD element (`hud-classes-special.md`).

**Data (script section 3; all 23 damagers).** `6_3_snow_base`: one cold zone (a box of **262.4 x 158.7 x 262.4 m** around the outdoor area) and 5 healing boxes (3.0 x 81.3 x 4.6, 28.3 x 6.7 x 30.9, 3.3 x 3.7 x 4.9, 6.9 x 5.9 x 8.0, 8.1 x 5.9 x 7.2 m); `6_4_alliance_headquarters`: one cold zone (**908.4 x 97.7 x 993.8 m**) and 15 healing boxes of 4 to 37 m. So **2 damaging and 21 healing**; the healing boxes are shelters. `ep_edtType` is 0 in all. Box sizes are as stored (relative to the damager's placement).

### 5. Alarms (read in outline; low)

`CAlarmEntity` is binary-only: **0 in the 59 worlds**, and no script in the data calls its only macro function. The editor does have the entity (a thumbnail `Alarm.tga` in the editor's content and the class names `SAlarmEntity` / `SAlarmProperties` registered for the editor): a designer would place one and call **`EmanateThreatSound`** from a script. That function calls the threat-sound emitter `samEmanateThreatSound(no emitter brain, the alarm entity, en_fHeardDistance)`: every brain in the world info's "brains without a foe" container whose position is **strictly closer than `en_fHeardDistance`** (a 3-D distance from the alarm's position) gets its puppet's threat-sound hook called, which sends it to look for a foe (`brain-senses.md` section 5.1 for the receiving side). `OnCreate` copies the record's `ep_fHeardDistance` into `en_fHeardDistance`; if the alarm's placement is unavailable the log says "Alarm entity returns invalid placement". Other emitters (shots, deaths, tracking a foe) call the same function with their own radius. So **a script "creates" an alarm only by the editor placing it and a macro calling `EmanateThreatSound`; the retail game does neither**.

## Invariants

1. `EDamageType` has 25 enumerators with the values of section 1; the killer's two constants 24 and 25 are `DMT_KILLER` and `DMT_ABYSS` in that table.
2. Every detector in the data is armed at start (`en_bActivated` 0 in 1,077 of 1,077, `mechanism-classes-triggers.md`).
3. The freezing numbers have one value set per parameter file (script section 5).

## Open questions

1. **What the damage entry does with types 24 and 25** (invulnerability, armour, god mode, the `BeInvulnerable` script command), the effect of a death type of 25 on the camera, and the `OnTrigger` of a prop target. **T01-308.**
2. **Which parameter file** (30 s or 60 s freezing) each player uses, the breath puffs and the freezing HUD. **T01-309.**
3. Run-time confirmation: killer periods, instant kill against god mode, freezing times. T01-174.

## Test vectors

1. **Killer, periodic.** Killer with kill-instantly 0, period 1, amount 10; an entity enters at t = 0 and stays: damage 10 (type 24) at about t = 1, 2, 3 ... (the first after one period, each at the first touch event at least 1 s after the previous); leaving at 1.5 and re-entering at 4: the stored time is 1.0, so damage **at once** on re-entry (4 - 1 > 1).
2. **Killer, instant.** `ep_bKillInstantly` 1, `ep_bAbyss` 1: a puppet touching it receives 2,147,483,647 of type 25 at every touch event; a barrel (non-puppet) is unaffected. With `ep_bKillInstantly` 0 and period 0: the same lethal damage, type 24.
3. **Freezing.** `ppp_fFreezeTime` 30, `ppp_fUnfrezeTime` 5, damage 1.5/s: standing in a cold zone for 30 s freezes fully; then 1 point of cold damage at 0.667 s intervals (30.67, 31.33, ... s); leaving after 15 s of freezing lowers the ratio at 1/5 per second (zero after 2.5 s); a healing box containing the player also unfreezes.
4. **Detector.** One-shot: first touch at t = 0: `Activated` raised once, hull removed, colour active; a script `Recharge` re-creates the hull and the inactive colour; custom detector with an empty list: category `ignoreall`, nothing happens; after `RegisterCustomDetection(player)`: category `custom_detector`; the player touching raises `CustomDetection` and the list is empty again.
