# Item classes: health, armor, treasure, extra life, repair and power-ups (T01-108, part of T01-026)

- **Roadmap task:** T01-108 (overview and the base class [`item-classes.md`](item-classes.md); companion [`item-classes-ammo-weapons.md`](item-classes-ammo-weapons.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, parents, member lists and the **values** of every placed item (script: heal, armor and money amounts, the flags, the power-up indices, the models; each equals the class constructor's default read from the binary: two independent readings); **medium** for the rules of sections 3 to 5 (each is one reading of a short function: the pickup effects, the deathmatch half rule, the power-up timing, the drops); **low** for "(name)" descriptions and the open questions. Nothing was run.

Prerequisites: [`item-classes.md`](item-classes.md) (the base class, the pickup protocol, the `.ep` respawn numbers, section numbers quoted below), `players-session-classes.md` (the inventory and the power-up index), `player-rules.md` (the power-up durations), `replication.md`.

## Summary

This page covers **54 classes (27 entity and properties pairs)**: health (`CHealthItemEntity` and 5 sizes), armor (`CArmorItemEntity` and 6 sizes), treasure (`CTreasureItemEntity` and 3 sizes), the extra life (`CLifeItemEntity`), the repair item (`CRepairItemEntity`) and the **power-ups** (`CPowerUpItemEntity` and 7 kinds). **52 are in the data** (only the strength power-up pair, `CStrengthItemEntity` and `CStrengthItemProperties`, is binary-only) and **1,523 of the 2,730 placed items** belong to them (health 687, armor 478, treasure 277, extra life 11, repair 5, power-ups 65; counts in section 1).

What a reader needs to know first:

1. **Size classes carry a constant.** A health, armor or treasure *size* class has no method of its own: its constructor sets the amount and a model, and the generic parent does the rest. **Health 5, 10, 25, 50, 100** (XS, S, M, L, XL), **armor 5, 10, 25, 50, 100, 200** (XS to XXL), **treasure 100, 1,000, 5,000** (S, M, L). These equal the numbers of the editor help (`help-digest.md`: armor 5/10/25/50/100/200, health 5/10/25/50/100) and every one of the 1,442 placed health, armor and treasure items (**high**, two readings). The `.ep` numbers of `player-rules.md` (10, 10, 25, 60, 120 ...) are respawn delays, not amounts (`item-classes.md` section 4).
2. **Health and armor are given by the puppet's own functions.** `OnPicked` asks the puppet `CanReceiveHealth(amount, flag)` / `CanReceiveArmor` and only then calls `ReceiveHealth` / `ReceiveArmor`; both clamp to 0 and the maximum, and in a **deathmatch** a pickup without the flag `en_bFillsAboveHalf` lifts the value only up to **half of the maximum** (section 3).
3. **The score of a pickup equals its amount** for health, armor, repair and treasure; the treasure is the score item (100, 1,000, 5,000 points). `CLifeItemEntity` scores 1,000 and adds a life; treasure and life are **not resurrected** by a chapter restore.
4. **Power-ups are timed** (60 s in single player, 20 to 40 s in multiplayer). The inventory stores a **start time** per power-up; a dropped power-up carries how long it was already active (`en_fWastedTime`), so the next picker gets the remainder. Seven classes, one per `PowerUpIndex`.
5. **The repair item heals a vehicle**, not the player: it needs and acts on the puppet the player is riding, with an amount that comes from its record (250 in the data, 100 by default).

## Evidence

- As [`item-classes.md`](item-classes.md): the export table of the 54 classes (symbols and named methods per class below), the vtables (every class has all 119 slots of `CBaseItemEntity`), the decompiled named methods and constructors, and `tools/re/item_classes_verify.py` for the data. The member tables of `SHealthItemEntity`, `SArmorItemEntity`, `STreasureItemEntity`, `CRepairItemEntity`/`Properties`, `CPowerUpItemEntity` and `CSeriousJumpItemEntity`/`Properties` were read (7 classes).
- **Constructor defaults and the data agree.** The 38 constructors of these classes (default form) were read for the amount, flag, power-up index and model path; the script lists the distinct values of every member over all 1,523 placed items. All items of a class carry one value per member (the single exception of editor-set flags is in `item-classes.md` section 6).
- **Second readings.** The power-up index and class: the constructors (`en_puiPowerUp`) and the factory `samCreatePowerUpItemProperties` (index to class) agree for all 7; `GetPickupScore` and `GetPickupAmount` of the size classes are one shared function that returns the amount; the timing of power-ups is read in three functions (`IsPowerUpActive`, `CPlayerInventory::ReceivePowerUp`, `DropPowerUp`) that agree.
- **Not checked.** The HUD and sound side of a pickup, the player-side score multiplier for "serious score", the effect of each power-up on the player (`players-session-classes.md` section 7.2 has the numbers read there), the editor's way of creating items.

## Layout / rules

### 1. The 54 classes

"Symbols (methods)" = exported symbols and named methods of the entity class (its properties class exports 7). "Items (worlds)" = placed items and the worlds they are in; "records" = property records (the extra ones belong to spawners).

| Class | Parent | Saved as | Members / files | Items (worlds) | Records | Symbols (methods) |
|---|---|---|---|---|---|---|
| `CHealthItemEntity`, `...Properties` | `CBaseItemEntity`, `CBaseItemProperties` | `SHealthItemEntity` 2 members (1 in 14 older worlds) / 57 `.wld` | | 687 (57) | 1,034 | 13 (6) |
| `CHealthItemXSEntity` | `CHealthItemEntity` | `SHealthItemXSEntity` | 0 / 24 | 141 (24) | 163 | 7 (0) |
| `CHealthItemSEntity` | | `SHealthItemSEntity` | 0 / 21 | 87 (21) | 124 | 7 (0) |
| `CHealthItemMEntity` | | `SHealthItemMEntity` | 0 / 50 | 255 (50) | 405 | 7 (0) |
| `CHealthItemLEntity` | | `SHealthItemLEntity` | 0 / 53 | 157 (53) | 262 | 7 (0) |
| `CHealthItemXLEntity` | | `SHealthItemXLEntity` | 0 / 33 | 47 (33) | 80 | 7 (0) |
| `CArmorItemEntity`, `...Properties` | `CBaseItemEntity` | `SArmorItemEntity` 2 members (1 in 13 older worlds) / 56 `.wld` | | 478 (56) | 742 | 13 (6) |
| `CArmorItemXSEntity` | `CArmorItemEntity` | `SArmorItemXSEntity` | 0 / 20 | 94 (20) | 123 | 7 (0) |
| `CArmorItemSEntity` | | `SArmorItemSEntity` | 0 / 29 | 144 (29) | 233 | 7 (0) |
| `CArmorItemMEntity` | | `SArmorItemMEntity` | 0 / 32 | 83 (32) | 124 | 7 (0) |
| `CArmorItemLEntity` | | `SArmorItemLEntity` | 0 / 43 | 83 (43) | 139 | 7 (0) |
| `CArmorItemXLEntity` | | `SArmorItemXLEntity` | 0 / 31 | 55 (31) | 92 | 7 (0) |
| `CArmorItemXXLEntity` | | **`CArmorItemXXLEntity`** (`C` name) | 0 / 13 | 19 (13) | 31 | 7 (0) |
| `CTreasureItemEntity`, `...Properties` | `CBaseItemEntity` | `STreasureItemEntity` 1 member / 32 `.wld` | | 277 (32) | 492 | 11 (4) |
| `CTreasureItemSEntity` | `CTreasureItemEntity` | `STreasureItemSEntity` | 0 / 18 | 110 (18) | 220 | 7 (0) |
| `CTreasureItemMEntity` | | `STreasureItemMEntity` | 0 / 30 | 128 (30) | 207 | 7 (0) |
| `CTreasureItemLEntity` | | `STreasureItemLEntity` | 0 / 26 | 39 (26) | 65 | 7 (0) |
| `CLifeItemEntity` | `CBaseItemEntity` | `SLifeItemEntity` | 0 / 11 | 11 (11) | 17 | 10 (3) |
| `CRepairItemEntity` | `CBaseItemEntity` | **`CRepairItemEntity`** | 1 / 1 | 5 (1) | 10 | 11 (4) |
| `CPowerUpItemEntity`, `...Properties` | `CBaseItemEntity` | **`CPowerUpItemEntity`** 2 members (1 in 9 older worlds) / 33 `.wld` | | 65 (33) | 107 | 9 (2) |
| `CStrengthItemEntity` | `CPowerUpItemEntity` | not saved | | 0 | | 7 (0) |
| `CSpeedPowerUpItemEntity` | | **same name** | 0 / 2 | 3 (2) | 3 | 7 (0) |
| `CSeriousJumpItemEntity` | | **same name** | 4 / 1 | 4 (1) | 8 | 8 (1) |
| `CSeriousDamageItemEntity` | | **same name** | 0 / 30 | 38 (30) | 63 | 8 (1) |
| `CSeriousScoreItemEntity` | | **same name** | 0 / 9 | 13 (9) | 26 | 7 (0) |
| `CInvulnerabilityPowerUpItemEntity` | | **same name** | 0 / 1 | 1 (1) | 1 | 7 (0) |
| `CInvisibilityPowerUpItemEntity` | | **same name** | 0 / 6 | 6 (6) | 6 | 7 (0) |

The properties class of each row has the same stem, the matching parent chain and 7 exported symbols (no named method), and 0 members, except `CSeriousJumpItemProperties` (4) and `CRepairItemProperties` (1). **Of the 54 classes, 52 are in the data** (binary-only here: `CStrengthItemEntity` and `CStrengthItemProperties`). The totals of the groups are health 687 + armor 478 + treasure 277 + extra life 11 + repair 5 + power-ups 65 = 1,523 items (the 2,730 minus the ammunition, weapon and generic items of the other pages: 927 + 228 + 52). `wld-entities.md` groups the 5 Klodovik ammunition items with the "others"; the totals agree.

### 2. Constructor defaults and data (health, armor, treasure, life, repair)

| Class | Amount (`en_slHealAmmount`, `en_slShieldAmmount`, `en_slMoney`) | `en_bFillsAboveHalf` | Model (`Models/Items/...`) | Respawn file / seconds |
|---|---|---|---|---|
| health XS | 5 | **1** | `Health/ExtraSmall_Item.mdl` | `healthitemxs.ep` 10 |
| health S | 10 | 0 | `Health/Small_Item.mdl` | 10 |
| health M | 25 | 0 | `Health/Medium_Item.mdl` | 25 |
| health L | 50 | 0 | `Health/Large_Item.mdl` | 60 |
| health XL | 100 | **1** | `Health/ExtraLarge_Item.mdl` | 120 |
| armor XS | 5 | **1** | `Armor/ExtraSmall_Item.mdl` | 10 |
| armor S | 10 | 0 | `Armor/Small_Item.mdl` | 10 |
| armor M | 25 | 0 | `Armor/Medium_Item.mdl` | 25 |
| armor L | 50 | 0 | `Armor/Large_Item.mdl` | 10 |
| armor XL | 100 | 0 | `Armor/ExtraLarge_Item.mdl` | 60 |
| armor XXL | 200 | **1** | `Armor/Super_Item.mdl` | 120 |
| treasure S | 100 | n/a | `Treasure/TreasureCoin_Item.mdl` | 60 |
| treasure M | 1,000 | n/a | `Treasure/TreasureBag_Item.mdl` | 60 |
| treasure L | 5,000 | n/a | `Treasure/TreasureChest_Item.mdl` | 60 |
| extra life | none (one life) | n/a | `PowerUps/ExtraLife_Item.mdl` | 60 (`lifeitem.ep`) |
| repair | record's `ep_slRepairAmount` (constructor default 100; **250 in all 10 records**) | n/a | `Health/Repair_Item.mdl` | 60 |

**Checks (script, high):** the amount of every one of the 1,442 placed health, armor and treasure items equals the row's value (for example 157 health L = 50, 128 treasure M = 1,000, 19 armor XXL = 200); the flag equals the row's value in every item that has the member (variants of the 14 older worlds omit it: default to the row); the model path of each item equals the class's path; the `.ep` link of each class is the one file of the row (2,140 of 2,730 items have the link). The three amounts match the editor help, a third source. The names (`GetName`, translated): health "Health", armor "Armor", treasure "Treasure"; the life and repair items return the empty name of the base class.

### 3. Health and armor: the rules (medium)

| Rule | Statement |
|---|---|
| `IsNeeded` | health: the puppet's `GetHealth` is **below** `GetMaxHealth`; armor: `GetArmor` **below** `GetMaxArmor` (the two puppet getters are slots of `CPuppetEntity`, found by the player's vtable) |
| `OnPicked` | `CanReceiveHealth(amount, flag)`; when it is true: the puppet's `ReceiveHealth(amount, flag)` (virtual) and then the base `OnPicked`. **When it is false the base `OnPicked` is not run**: the item is neither hidden nor scored (it can only happen in a deathmatch, below). Armor is the same with `CanReceiveArmor` / `ReceiveArmor` |
| `CanReceiveHealth(amount, flag)`, `CanReceiveArmor` | true when the session is **not** a deathmatch or `flag` is 1; in a deathmatch with `flag` 0: true only while the current value is **below half of the maximum** |
| `ReceiveHealth(amount, flag)`, `ReceiveArmor` | on the host or locally only (a client does nothing): *new = clamp(value + amount, 0, maximum)*; in a deathmatch with `flag` 0 the result is further capped at **max(old value, maximum / 2)** (integer halves), never below 0. So a normal pickup cannot take a deathmatch player above half of the maximum, and an XS, XL (health) or XS, XXL (armor) pickup, the ones with the flag, can fill to the maximum |
| `GetPickupAmount`, `GetPickupScore` | both return the amount member |
| `OnlyFirstPlayerPicks` | in co-op: `!sgp_bHealthAndArmorStayInCoop`; otherwise the base rule (`item-classes.md` rule 5) |

The maximum is the puppet's own `en_slHealthMax` / `en_slArmorMax` (the player's nominal health is 100, `players-session-classes.md`); the same two functions serve every puppet, so enemies obey the same clamp (not used by items). The flag's name, `en_bFillsAboveHalf`, is the binary's own; the reading above is what the functions do with it (**medium**).

### 4. Treasure, extra life and repair (medium)

- **Treasure** (`CTreasureItemEntity`): no `IsNeeded`/`OnPicked` override: always needed, the base pickup, **score = money** (100, 1,000 or 5,000), `en_bOnlyFirstPlayerPicks` 1 (constructor default of its properties and all 277 items). `IsChapterResurrectionEnabled` is false: a taken treasure stays taken after a chapter restore. 
- **Extra life** (`CLifeItemEntity`): base pickup, **score 1,000** (which itself counts towards the extra-life score), and on the host `CGameInfoEntity::OnAddLife(0)`, which calls the player's `OnAddLife` for **every one of the 17 player slots**: all players get a life (the player's `OnAddLife` increments its life count, plays the extra-life sound for a local player, only when lives are in use: game type single or co-op and `sgp_bUseLives`, and never in a deathmatch). Chapter resurrection is off. The score route is the same function: every `ggp_slExtraLifeScore` (10,000) points of the session score call `OnAddLife(1)` once per threshold crossed, for all players (`CGameInfoEntity::ReceiveScore`: skipped in a deathmatch and while a flag of the game statistics, probably the frozen-statistics flag, is set).
- **Repair** (`CRepairItemEntity`): `OnCreate` copies the amount from the record. `IsNeeded`: the player has a **ride** (the puppet the player rides; its handle is a member of the puppet, name not resolved) and that puppet's health is below its maximum. `OnPicked`: the base pickup, then the ride's `ReceiveHealth(amount, 1)` (so no deathmatch half rule). Data: **5 items in 1 world, amount 250** (record and entity). Score = amount. (The repair item's class is saved under its `C` name.)

### 5. Power-ups

**Classes and indices.** `CPowerUpItemEntity` has two members: `en_puiPowerUp` (a `PowerUpIndex`) and `en_fWastedTime` (replicated; data `.1`, 0 in all 65 placed power-ups). The index of each class is set by its constructor and by the factory `samCreatePowerUpItemProperties` (two readings): **0 strength** (`CStrengthItemEntity`, not placed), **1 speed** (`CSpeedPowerUpItemEntity`), **2 jump** (`CSeriousJumpItemEntity`), **3 damage** (`CSeriousDamageItemEntity`), **4 score** (`CSeriousScoreItemEntity`), **5 invulnerability** (`CInvulnerabilityPowerUpItemEntity`), **6 invisibility** (`CInvisibilityPowerUpItemEntity`); in the data `en_puiPowerUp` is 1 to 6, equal to the class's constructor value in all 65 items (**high**). Models: `PowerUps/SeriousStrength_Item.mdl`, `SeriousSpeed_Item`, `SeriousJump_Item`, `SeriousDamage_Item`, `SeriousScore_Item`, `Invulnerability_Item`, `Invisibility_Item`.

| Class | Placed | `en_bOnlyFirstPlayerPicks` (entity) | Respawn (`.ep`, s) | Own method |
|---|---|---|---|---|
| strength | 0 | n/a | 60 | none |
| speed | 3 in 2 worlds | 0 | 40 | none |
| jump | 4 in 1 world | **1** | 40 | `OnCreate` copies the four numbers below |
| damage | 38 in 30 worlds | **1** (forced: `OnCreate` sets the record's flag to 1, and so does the properties constructor) | 40 | `OnCreate` |
| score | 13 in 9 worlds | 0 | 40 | none |
| invulnerability | 1 | 0 | 60 | none |
| invisibility | 6 in 6 worlds | 0 | 40 | none |

**`CSeriousJumpItem`.** Members (data order, binary names): `en_fJumpMultiplier`, `en_fAirControlPeriod`, `en_fAirControlFactor`, `en_fAirControlVelocityMultiplier`. Constructor defaults **5, 5, 0.5, 1**; the 4 placed items (and 8 records) carry **3.8, 5, 0.5, 2.2** (script). On pickup the receiving machine copies the four numbers of the item **into the player's four `en_fJumpPowerup...` members** (`players-session-classes.md` section 7.2), which replace the jump multiplier and air-control values while the power-up is active (**medium**).

**Pickup.** `IsNeeded` is the base one (always true: a power-up can always be taken, even while active). `OnPicked`: the base pickup (score **50**), then the player's `ReceivePowerUp(item, index, en_fWastedTime)`. `ReceivePowerUp` is a **server-to-clients call** (`replication.md`): on the host it is sent to all machines; on arrival it calls the inventory's `ReceivePowerUp(index, wasted)` and, for index 2 only, the jump copy above.

**Timing (medium; three functions agree).** The inventory keeps **one start time per power-up** (`pi_atmPowerUp`). `ReceivePowerUp(index, wasted)` sets *start = now - wasted* and keeps it **only if it is later than the stored start** (so taking a power-up again restarts it; taking one with much time already wasted does not shorten an active one; index -1 is ignored). `IsPowerUpActive(index)`: *start <= now < start + duration(index)*, where `duration` is `samGetPowerUpDuration(index)`: the table `ggp_pudPowerUpDurations` (single player and co-op) or `ggp_pudPowerUpDurationsDM` (deathmatch), seven floats in index order (values in `player-rules.md`: 60 s each in single player; 40, 20, 40, 30, 40, 30, 30 in multiplayer; 0 for an invalid index). All start times are reset to 1,000 s before time zero, so no power-up is active. **This corrects the wording of `players-session-classes.md` ("end time now + seconds")**: the stored value is a start time and the third argument is the time already used.

**Drops.** When a player dies (and on request) `DropActivePowerUps` drops every active power-up (7 indices are tested) and then resets them all (a call to all machines). `DropPowerUp(index)`: `CreatePowerUpItemEntity` (factory, spawn at the player's placement, `en_bDynamicallyCreated` = 1), a random velocity along the player's view direction plus a fixed upward and sideways part, **`en_fWastedTime` = now - start** (the time used so far), and a `Delete` event after `ggp_fPowerUpItemStayTimeAfterDrop` (10 s). The picker then starts with that much already wasted. The dropped item is a `CPowerUpItemEntity` child of the right kind (all 7 can occur, including strength; the jump kind drops with the default numbers 5, 5, 0.5, 1, not the placed item's).

## Invariants

1. Health, armor, treasure and life classes have no method but those of section 3 and 4; the 14 size classes (5 + 6 + 3) and the 5 power-ups without a method add only a constructor.
2. A placed health, armor, treasure or repair item's amount is the class constant (repair: the record's number); no item in the retail data deviates (1,447 items checked).
3. Health 5/10/25/50/100, armor 5/10/25/50/100/200, treasure 100/1,000/5,000; `GetPickupScore` equals the amount of these classes.
4. A power-up index is 0 to 6; the item class for an index is the one of section 5; the stored time is a start time; a power-up is active for `duration(index)` seconds (60 in single player).
5. A deathmatch pickup of health or armor without the flag cannot raise the value above half of the maximum (or above its old value if that was higher).
6. Treasure and extra life are not restored by a chapter restore; every other item is.

## Open questions

1. The remaining flow details of the base pickup (events `Picked`/`OnePicked`, scheduling, chapter resurrection) and the creation function: **T01-160**, **T01-161** (see [`item-classes.md`](item-classes.md)).
2. **Health and armor for non-player puppets and the half rule:** `en_bFillsAboveHalf` as the designers meant it, the player's maximum health and armor at start (the player's `GetInitialMaxHealth` is 200, nominal 100, `players-session-classes.md`), the "Tourist" health recharge; the **ride** handle of the repair item. **T01-162.**
3. **The power-up effects** (strength, speed multiplier 2 in `ppp_fSpeedPowerUpMultiplier`, damage 4 and 2.5, score multiplier 5, invulnerability, invisibility) and the HUD (`CPowerUpHudElement`) belong to the player rules: `players-session-classes.md`, **T01-114**, **T01-111**.
4. **Run-time checks** of amounts, the deathmatch half rule, respawn delays, power-up durations and the dropped remaining time: **T01-163**.
5. Whether the strength power-up can be placed or created by anything but a drop: **T01-164**.

## Test vectors

1. **Amounts.** Health L gives 50, armor XXL 200, treasure L 5,000; `GetPickupScore` returns the same numbers (health 25 -> score 25).
2. **Deathmatch half rule.** Maximum 100, health 30, a health M pickup (25, flag 0): `CanReceiveHealth` true (30 < 50), result clamp(55) then capped at max(30, 50) = **50**. Health 60: `CanReceiveHealth` false (60 is not below 50): the item is not taken. A health XL (flag 1) at health 30 gives 100 (the clamp only).
3. **Single player.** The same pickup at health 30 gives 55 (no half rule); at 95 gives 100 (clamp); at 100 `IsNeeded` is false and the player does not take it.
4. **Power-up timing.** Single player, `Serious damage` duration 60: taken at time *t* with wasted 0 it is active for *t* <= now < *t* + 60. Dropped by a player who had used 20 s of it, it carries wasted 20 and the picker at time *u* gets start *u* - 20: it ends at *u* + 40. Taking a second damage power-up with wasted 0 at *t* + 30 moves the start to *t* + 30 (later), so it lasts to *t* + 90.
5. **Jump power-up.** The placed item has 3.8, 5, 0.5, 2.2; after the pickup the player's `en_fJumpPowerupJumpMultiplier` is 3.8, the air-control period 5, the factor 0.5 and the velocity multiplier 2.2 while the power-up is active.
6. **Life.** In a single-player session with lives on, picking up an extra life adds one life to **each** of the 17 slots' players and adds 1,000 to the score; a treasure M adds 1,000 to the score only; crossing a multiple of 10,000 score gives one more life.
7. **Data.** The script prints 157 `SHealthItemLEntity` with amount 50 in 53 worlds; 94 `SArmorItemXSEntity` with amount 5 and flag 1 (87 with the member); 4 `CSeriousJumpItemEntity` with 3.8, 5, 0.5, 2.2.
