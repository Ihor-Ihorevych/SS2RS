# Item classes: ammunition, ammunition packs and weapon pickups (T01-108, part of T01-026)

- **Roadmap task:** T01-108 (overview and the base class [`item-classes.md`](item-classes.md); companion [`item-classes-vitals.md`](item-classes-vitals.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, parents, member lists, the **ammunition kind, weapon index and pickup quantity of every placed item** and the contents of the 665 ammunition containers (script; the kind, index and quantity equal the class constructor's default read from the binary: two independent readings); **medium** for the amounts of ammunition pickups (each is the constant returned by one small function, with the editor help as a partial third source), the pickup rules and the factories (one reading each); **low** for "(name)" descriptions and the open questions. Nothing was run.

Prerequisites: [`item-classes.md`](item-classes.md) (the pickup protocol, respawn numbers, section numbers quoted below), `players-session-classes.md` (the enumerations `WeaponIndex` and `AmmoIndex`, the ammunition caps and the enemy multiplier, the table weapon to ammunition kind), `weapon-classes.md` (section 8: `CWeaponItemEntity`, the cannon and combo pickups), `projectile-classes.md` (the bullets pickup), `save-game.md` (the chapter's ammunition container).

## Summary

This page covers **48 classes (24 entity and properties pairs)**: the ammunition base `CAmmoItemEntity`, the **ammunition pack** `CAmmoPackItemEntity`, eight **live ammunition kinds** (`CGrenadesItemEntity`, `CHandGrenadesItemEntity`, `CKlodovikItemEntity`, `CNapalmItemEntity`, `CPlasmaItemEntity`, `CRocketsItemEntity`, `CShellsItemEntity`, `CSniperBulletsItemEntity`) and **14 live weapon pickups** (`CAutoShotgunItemEntity`, `CBeamGunItemEntity`, `CColtItemEntity`, `CDoubleShotgunItemEntity`, `CFlamerItemEntity`, `CGrenadeLauncherItemEntity`, `CKlodovikGunItemEntity`, `CMiniGunItemEntity`, `CPlasmaRifleItemEntity`, `CRocketLauncherItemEntity`, `CSeriousBombItemEntity`, `CSniperItemEntity`, `CUziItemEntity`, `CZapGunItemEntity`). The bullets, cannon-ball, cannon and combo pickups are in `projectile-classes.md` and `weapon-classes.md`; the three obsolete ones (blades, serious bomb as ammunition, circular saw pickup) are on the overview page. **40 of the 48 classes are in the data** (binary-only: the Colt, flamer, zap gun and napalm pairs) and **946 of the 2,730 placed items** belong to them (ammunition 733 of 8 kinds including 78 packs, weapon pickups 213 of 11 kinds; with the bullets and cannon-ball pickups (194) and the cannon pickups (15) of the other pages the family has 927 ammunition and 228 weapon items).

What a reader needs to know first:

1. **An ammunition class is a (kind, amount) pair.** The kind is a member (`ai_aiAmmoIndex`, the data's only member of the base) set by the class constructor; the **amount is a constant returned by `GetPickupAmount`** of the class: **grenades 10, hand grenades 6, Klodovik 1, napalm 50, plasma 50, rockets 8, shells 20, sniper bullets 5** (and bullets 100, cannon balls 4 on the other pages). The editor help and `player-rules.md` could not give these because they are in code (**medium**: one small function each). `ReceiveAmmo` then multiplies by the session's enemy multiplier (`players-session-classes.md` section 7.3).
2. **A weapon pickup is a (weapon index, quantity) pair.** The weapon index (`en_wiWeaponIndex`) and the ammunition quantity (`en_slAmmoQuantity`, the data's number `.1`) are set by the class constructor and are **the same in every placed item** of the class: auto shotgun 2 / 10, double shotgun 3 / 20, Uzi 4 / 100, minigun 5 / 200, rocket launcher 6 / 8, grenade launcher 7 / 10, plasma rifle 8 / 50, Klodovik gun 9 / 1, serious bomb 11 / 1, sniper 13 / 5, beam gun 15 / 50, flamer 16 / 200 (cannon 10 / 4 on the weapon page). **High** (script against the constructors). This is the table `weapon-classes.md` open question 6 asked for (the ammunition amounts of the pickups); the table index to parameter file stays with T01-122.
3. **An ammunition pack is a list.** `CAmmoPackItemEntity` has the ammunition kind "none" and an owned `CAmmoItemContainer` (a list of `CAmmoItem`: ammunition kind and quantity); its pickup gives every entry. The data has **229 pack containers** (78 in placed items, 151 in records) with 0 to 10 entries each; the same container class is the chapter's start ammunition (436 in 218 chapter-info entities and records).
4. **Weapons without an ammunition kind are never dropped**, which is why the Colt and the zap gun pickups exist as classes but no world and no drop makes one; the flamer pickup (ammunition napalm) is made only by a drop.
5. **Two bombs.** The weapon pickup `CSeriousBombItemEntity` (index 11, ammunition `AI_SERIOUS_BOMB`, quantity 1) is the live one; the ammunition class `CSerious_BombItemEntity` is obsolete. The pickup is hidden at boot and at simulation start when serious bombs are disabled (co-op with `sgp_bDisableSeriousBombs`), and it is forced to "only the first player picks".

## Evidence

- As [`item-classes.md`](item-classes.md). Member tables read: `SAmmoItemEntity`, `SAmmoPackItemEntity`/`Properties`, `SWeaponItemEntity`. Constructors of all 24 entity classes of this page were decompiled (kind or index, quantity, model path), the 8 + 1 small `GetPickupAmount` functions, `GetName` (24 translated names), `IsNeeded`, `OnPicked`, `OnlyFirstPlayerPicks` of the ammunition base, the pack's `OnCreate`, `OnDelete`, `OnPicked`, the weapon base's `OnCreate`, `IsNeeded`, `OnPicked`, `OnlyFirstPlayerPicks`, `GetPickedUpAmmo`, the serious bomb's five overrides and the two factories `samCreateWeaponItemProperties` and `samGetAmmoForWeapon`, `DropWeapon`, `CreateWeaponItemEntity`.
- **Second readings.** Ammunition index: the constructor and the data's `ai_aiAmmoIndex` of every item; weapon index and quantity: constructor, data, and `samCreateWeaponItemProperties` (index to class, the same 14 + cannon); the amounts of treasure-like size classes were checked on the vitals page the same way, the ammunition amounts only by the function (the data has no amount: the sniper-bullet function is a function shared with `CAmmoItemEntity::GetPickupScore`, which is how the score 5 was found).
- **Data.** `tools/re/item_classes_verify.py` section 4 prints, per class, the placed items with the values of their members and, over all 665 containers, the quantity of every ammunition kind and the entries per container. **Not checked:** the player-side `ReceiveWeapon`/`ReceiveAmmo` (inventory spec), the weapon parameter table, nothing was run.

## Layout / rules

### 1. The 48 classes

"Items (worlds)" = placed items; "records" = property records (the extra ones belong to spawners); symbols as on the other pages.

| Class (entity; properties has the same stem) | Parent | Saved as | Members / files | Items (worlds) | Records | Symbols (methods) |
|---|---|---|---|---|---|---|
| `CAmmoItemEntity` | `CBaseItemEntity` | `SAmmoItemEntity` | 1 / 56 | the children below | | 12 (5) |
| `CAmmoPackItemEntity` | `CAmmoItemEntity` | `SAmmoPackItemEntity` | 1 / 25 | 78 (25) | 151 | 11 (4) |
| `CGrenadesItemEntity` | `CAmmoItemEntity` | `SGrenadesItemEntity` | 0 / 14 | 39 (14) | 54 | 9 (2) |
| `CHandGrenadesItemEntity` | | `SHandGrenadesItemEntity` | 0 / 39 | 135 (39) | 213 | 9 (2) |
| `CKlodovikItemEntity` | | `SKlodovikItemEntity` | 0 / 4 | 5 (4) | 10 | 9 (2) |
| `CPlasmaItemEntity` | | `SPlasmaItemEntity` | 0 / 14 | 44 (14) | 53 | 9 (2) |
| `CRocketsItemEntity` | | `SRocketsItemEntity` | 0 / 37 | 125 (37) | 189 | 9 (2) |
| `CShellsItemEntity` | | `SShellsItemEntity` | 0 / 47 | 271 (47) | 399 | 9 (2) |
| `CSniperBulletsItemEntity` | | `SSniperBulletsItemEntity` | 0 / 17 | 36 (17) | 46 | 9 (2) |
| `CNapalmItemEntity` | | not saved | | 0 | | 9 (2) |
| `CAutoShotgunItemEntity` | `CWeaponItemEntity` | `SAutoShotgunItemEntity` | 0 / 17 | 25 (17) | 30 | 8 (1) |
| `CDoubleShotgunItemEntity` | | `SDoubleShotgunItemEntity` | 0 / 23 | 41 (23) | 48 | 8 (1) |
| `CUziItemEntity` | | `SUziItemEntity` | 0 / 15 | 17 (15) | 24 | 8 (1) |
| `CMiniGunItemEntity` | | `SMiniGunItemEntity` | 0 / 15 | 16 (15) | 22 | 8 (1) |
| `CRocketLauncherItemEntity` | | `SRocketLauncherItemEntity` | 0 / 24 | 31 (24) | 39 | 8 (1) |
| `CGrenadeLauncherItemEntity` | | `SGrenadeLauncherItemEntity` | 0 / 12 | 12 (12) | 15 | 8 (1) |
| `CPlasmaRifleItemEntity` | | `SPlasmaRifleItemEntity` | 0 / 10 | 10 (10) | 13 | 8 (1) |
| `CKlodovikGunItemEntity` | | `SKlodovikGunItemEntity` | 0 / 4 | 5 (4) | 8 | 8 (1) |
| `CSeriousBombItemEntity` | | `SSeriousBombItemEntity` | 0 / 19 | 34 (19) | 62 | 13 (6) |
| `CSniperItemEntity` | | `SSniperItemEntity` | 0 / 16 | 17 (16) | 23 | 8 (1) |
| `CBeamGunItemEntity` | | `SBeamGunItemEntity` | 0 / 5 | 5 (5) | 5 | 8 (1) |
| `CColtItemEntity` | | not saved | | 0 | | 8 (1) |
| `CFlamerItemEntity` | | not saved | | 0 | | 8 (1) |
| `CZapGunItemEntity` | | not saved | | 0 | | 8 (1) |

(The placed-item sums are 733 ammunition and 213 weapon items on this page; with the bullets, cannon-ball and cannon pickups 927 and 228; `wld-entities.md` quotes 922 for ammunition, a grouping that differs by 5 items, and the totals agree at 2,730.) All 24 properties classes have the parent of the same chain (`CAmmoItemProperties`, `CWeaponItemProperties`), 7 exported symbols, 0 members, except `CAmmoPackItemProperties` (1: `ep_hcaiPackAmmo`).

### 2. Ammunition: kinds, amounts, names, models (constructor and data)

| Class | `AmmoIndex` | Amount (`GetPickupAmount`) | Model (`Models/Items/Ammo/`) | Name (translated) | Respawn |
|---|---|---|---|---|---|
| `CGrenadesItemEntity` | 3 `AI_GRENADES` | 10 | `Grenades_Item.mdl` | Grenades | 30 |
| `CHandGrenadesItemEntity` | 4 `AI_HAND_GRENADES` | 6 | `Hand_Grenades_Item.mdl` | Hand grenades | 30 |
| `CKlodovikItemEntity` | 5 `AI_KLODOVIK` | 1 | `Klodovik_Item.mdl` | Klodovik | 30 |
| `CPlasmaItemEntity` | 6 `AI_PLASMA` | 50 | `Plasma_Item.mdl` | Plasma | 30 |
| `CRocketsItemEntity` | 7 `AI_ROCKETS` | 8 | `Rockets_Item.mdl` | Rockets | 30 |
| `CShellsItemEntity` | 9 `AI_SHELLS` | 20 | `Shells_Item.mdl` | Shells | 30 |
| `CSniperBulletsItemEntity` | 10 `AI_SNIPER_BULLETS` | 5 | `SniperBullets_Item.mdl` | Sniper bullets | 30 |
| `CNapalmItemEntity` (binary only) | 11 `AI_NAPALM` | 50 | `Napalm_Item.mdl` | Napalm | 30 |
| `CBulletsItemEntity` (`projectile-classes.md`) | 1 `AI_BULLETS` | 100 | `Bullets_Item.mdl` | Bullets | 30 |
| `CCannonBallsItemEntity` (`weapon-classes.md`) | 2 `AI_CANNON_BALLS` | 4 | `CannonBalls_Item.mdl` | Cannon balls | 30 |
| `CAmmoPackItemEntity` | -1 (none) | the container (section 3) | `AmmoPack_Item.mdl` | Ammo pack | 30 |

**Data (script, high):** `ai_aiAmmoIndex` of every placed ammunition item equals its class's value (shells 271 x 9, hand grenades 135 x 4, bullets 151 x 1, rockets 125 x 7, plasma 44 x 6, cannon balls 43 x 2, grenades 39 x 3, sniper bullets 36 x 10, Klodovik 5 x 5, ammo pack 78 x -1); no placed item uses blades (0), serious bomb (8) or napalm (11). The amount has no data counterpart. The amounts are the values `ReceiveAmmo` receives; the inventory multiplies them by the enemy multiplier and clamps to the multiplied cap (`players-session-classes.md`): at multiplier 1 a shells pickup gives 20 up to the cap 200.

### 3. The ammunition base and the pack (medium)

| Function | What it does |
|---|---|
| `CAmmoItemEntity::IsNeeded` | the player's ammunition count of this kind is **below its maximum** (the inventory's `GetAmmoQuantity < GetMaxAmmoQuantity`; the maximum includes the enemy multiplier) |
| `OnPicked` | the player's `ReceiveAmmo(kind, GetPickupAmount())` (host only inside), then the base pickup |
| `OnlyFirstPlayerPicks` | co-op: `!sgp_bAmmoStaysInCoop`; otherwise the base rule |
| `GetPickupScore` | **5** (shared stub); `GetPickupAmount`: the constant of the class, 0 in the base |
| `CAmmoPackItemEntity::OnCreate` | base creation, then **a deep copy** of the container of the record (`ep_hcaiPackAmmo`) into the entity's own member (`en_hcaiPackAmmo`, owned); `OnDelete` deletes that copy |
| `CAmmoPackItemEntity::OnPicked` | the ammunition base's pickup with kind "none" (gives nothing), then **for every `CAmmoItem` of the container: `ReceiveAmmo(kind, quantity)`** |
| pack `IsNeeded` | the base ammunition rule with kind "none": the maximum of an out-of-range kind is 0 and the count is -1, so the pack is **always needed** (derived from `players-session-classes.md` section 7.3: **medium-low**) |

**The container (script, high).** `CAmmoItemContainer` has one member, a list of `CAmmoItem` (two members: `ai_aiAmmoIndex` and `ai_slQuantity`; base `CScriptObject`). 665 containers: 78 (placed pack entities) + 151 (pack records) + 218 + 218 (chapter-info entity and record, `en_hcaiCustomAmmoItemsAddedToPlayer`; the player start ammunition, `save-game.md`). Over all 665 containers there are **258 distinct contents**. An entry exists for kinds bullets, cannon balls, grenades, hand grenades, Klodovik, plasma, rockets, serious bomb, shells and sniper bullets; **blades and napalm never occur**. The quantities of a pack range, per kind: bullets 50 to 1,200, shells 50 to 450, rockets 2 to 50, hand grenades 4 to 32, grenades 5 to 50, plasma 100 to 800, cannon balls 3 to 30, Klodovik 5 to 15, sniper bullets 0 to 50, serious bomb 1 (3 times); the full per-quantity tables are the script's output. A pack carries **1 to 9 entries** (6, 12, 30, 38, 55, 48, 6, 13, 21 containers for 1 to 9 entries); a chapter container 0 to 10.

### 4. Weapon pickups (medium; weapon page for `CWeaponItemEntity`)

The base `CWeaponItemEntity` (`weapon-classes.md` section 8) supplies `OnCreate` (it looks the two weapon parameter files up by index: `samGetWeaponParamsPath` and `samGetWeaponRenderParamsPath` return a path for the indices 0 to 13, 15 and 16 and an empty string for 14, the combo), `IsNeeded`, `OnPicked`, `OnlyFirstPlayerPicks` (co-op: `!sgp_bWeaponStaysInCoop`), `GetPickedUpAmmo` (the quantity member), score **100** and a dynamic shadow. Read here once more: `IsNeeded` is true when the player **does not own the weapon**, otherwise while its ammunition of the weapon's kind is below the maximum; `OnPicked` runs the base pickup, then the player's `ReceiveWeapon(index)`, then `ReceiveAmmo(samGetAmmoForWeapon(index), GetPickedUpAmmo())`. The 14 classes below override only `GetName` (and the serious bomb five more).

| Class | `WeaponIndex` | Quantity | Ammunition kind used | Model (`Models/Weapons/`) | Name | Respawn |
|---|---|---|---|---|---|---|
| `CZapGunItemEntity` (binary only) | 1 | not set (0) | none | `ZapGun/ZapGun_Item.mdl` | Zap gun | 10 |
| `CAutoShotgunItemEntity` | 2 | 10 | shells | `AutoShotgun/AutoShotgun_Item.mdl` | Auto shotgun | 10 |
| `CDoubleShotgunItemEntity` | 3 | 20 | shells (2 per shot) | `DoubleShotgun/DoubleShotgun_Item.mdl` | Double shotgun | 10 |
| `CUziItemEntity` | 4 | 100 | bullets | `Uzi/Uzi_Item.mdl` | Uzi | 10 |
| `CMiniGunItemEntity` | 5 | 200 | bullets | `Minigun/Minigun_Item.mdl` | Minigun | 10 |
| `CRocketLauncherItemEntity` | 6 | 8 | rockets | `RocketLauncher/RocketLauncher_Item.mdl` | Rocket launcher | 10 |
| `CGrenadeLauncherItemEntity` | 7 | 10 | grenades | `GrenadeLauncher/GrenadeLauncher_Item.mdl` | Grenade launcher | 10 |
| `CPlasmaRifleItemEntity` | 8 | 50 | plasma | `PlasmaRifle/PlasmaRifle_Item.mdl` | Plasma rifle | 10 |
| `CKlodovikGunItemEntity` | 9 | 1 | Klodovik | `Klodovik/Klodovik_Item.mdl` | "Klodovik" (the ammunition's name) | 10 |
| (`CCannonItemEntity`, `weapon-classes.md`) | 10 | 4 | cannon balls | `Cannon/Cannon_Item.mdl` | Cannon | 30 |
| `CSeriousBombItemEntity` | 11 | 1 | serious bomb | `SeriousBomb/SeriousBomb_Item.mdl` | Serious bombs | 10 |
| `CColtItemEntity` (binary only) | 12 | not set (0) | none | `Colt/Colt_Item.mdl` | Colt | 10 |
| `CSniperItemEntity` | 13 | 5 | sniper bullets | `Sniper/Sniper_Item.mdl` | Sniper | 10 |
| `CBeamGunItemEntity` | 15 | 50 | plasma | `BeamGun/BeamGun_Item.mdl` | Beam gun | 10 |
| `CFlamerItemEntity` (binary only) | 16 | 200 | napalm | `Flamer/Flamer_Item.mdl` | Flamer | 10 |

**Data (script, high):** the weapon index of every placed pickup equals its class's (12 classes + the cannon); the quantity equals the class value in every pickup that has the member (14 older worlds lack it; the cannon 14 of 15, the minigun 13 of 16); `en_pbwpWeaponParams` and `en_pbwpWeaponRenderParams` are set in all of them and `bip` file is one per class. The ammunition kind of the third column is the table of `players-session-classes.md` section 7.3 (the beam gun uses plasma, which is why its pickup is 50).

**Factory (second reading, medium).** `samCreateWeaponItemProperties(index)` builds the properties of the pickup class for the indices 1 to 13, 15 and 16 (**no case for 0 and 14**: the circular saw pickup class is obsolete, the combo weapon is not a weapon); `CPlayerPuppetEntity::CreateWeaponItemEntity` uses it for drops. `DropWeapon(index)` makes a pickup **only if `samGetAmmoForWeapon(index)` is not none** (so never for the saw, zap gun, Colt and combo) and sets its quantity to the player's **current** count of that ammunition kind; it stays `ggp_fWeaponItemStayTimeAfterDrop` (10 s). `DropCurrentWeapons` drops the weapon of each hand (once, if both are the same) and stores the weapon tools.

**Serious bomb pickup (medium).** The properties constructor and `OnCreate` set `ep_bOnlyFirstPlayerPicks` to 1 (all 34 items and 62 records have it); `IsNeeded` replaces the weapon rule by "the player's serious-bomb ammunition is below its maximum" (so the player does not need to own the weapon: the ammunition count 0 is below the cap); `OnBoot` and `OnSimStart` hide the item while `samIsSeriousBombsDisabled` (co-op and `sgp_bDisableSeriousBombs`); `GetPickupAmount` returns 1. (The decompiler lost the branch structure of the two hooks: the hiding rule is read from their name and the helper's name, **medium-low**.)

## Invariants

1. A placed ammunition item's `ai_aiAmmoIndex` equals the class's kind (10 classes, 733 items on this page and 194 more on the other pages: the script prints each); the pack's is -1.
2. A placed weapon pickup's index is the class's; its quantity is the class constant; quantity, index, params links are set in every pickup of the 37 newer worlds (9 older worlds lack the quantity).
3. Ammunition amounts: grenades 10, hand grenades 6, Klodovik 1, napalm 50, plasma 50, rockets 8, shells 20, sniper bullets 5, bullets 100, cannon balls 4; score 5 each; respawn 30 s.
4. Weapon pickup score 100, respawn 10 s (cannon 30 s).
5. No world places `CColtItemEntity`, `CFlamerItemEntity`, `CZapGunItemEntity` or `CNapalmItemEntity`; weapons without an ammunition kind are never dropped.
6. A container entry's kind is an `AmmoIndex` 0 to 11 and in the data 1 to 10; quantities are non-negative integers (0 occurs).

## Open questions

1. The pack's and the weapon pickup's behaviour in co-op beyond the stay options, and the exact interplay of `ReceiveAmmo` with the enemy multiplier at a pickup (the co-op term): **T01-115** (`players-session-classes.md`), **T01-122** (the weapon index to parameter-file table and the combo pickup: **answered in `weapon-pickups.md`**, which also gives the receive and co-op stay rules).
2. The base pickup flow, creation and tool use: **T01-160**, **T01-161**; run-time checks of every amount, score and respawn here: **T01-163**.
3. Whether `AI_BLADES` and `AI_NAPALM` pickups can be made at all (the obsolete blades class; no creator of the napalm pickup was found): **T01-164**.
4. The editor-side catalogue of weapon pickups (the help lists 15; the live classes here are 14 + the cannon): **T01-164**.

## Test vectors

1. **Amounts.** A rockets pickup in single player at multiplier 1 gives 8 rockets (cap 50); two give 16; at 49 rockets the count becomes 50 (clamp) and at 50 `IsNeeded` is false. A shells pickup gives 20.
2. **Weapon pickup.** `CUziItemEntity`: index 4, quantity 100. A player without the Uzi takes it (`IsNeeded` true), receives the Uzi and 100 bullets; a player who has the Uzi and 1,000 bullets (the cap) does not take it.
3. **Serious bomb.** The pickup (index 11, quantity 1) is needed by a player with 0 bombs even if the player already owns the weapon; in co-op with `sgp_bDisableSeriousBombs` the items are hidden.
4. **Drop.** A player dying with the rocket launcher and 12 rockets drops a `CRocketLauncherItemEntity` with quantity 12 (not 8) that deletes itself after 10 s; a player dying with only the Colt drops nothing.
5. **Pack.** A pack whose container holds (shells 100) gives 100 shells and nothing else; one with (bullets 300, plasma 200, rockets 5, shells 200) gives the four (script: 6 pack containers have exactly these contents).
6. **Data.** 271 `SShellsItemEntity` in 47 worlds all with kind 9; 78 `SAmmoPackItemEntity` with kind -1; 34 `SSeriousBombItemEntity` all with `en_bOnlyFirstPlayerPicks` 1.
