# Weapon pickups, the weapon tables and the combo weapons (T01-122)

- **Roadmap task:** T01-122 (follow-up of `weapon-classes.md` open question 6; with `item-classes-ammo-weapons.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/weapon_pickups_verify.py` reproduces from the data: the 16 weapon files and rendering files that the index table names, the leftover files, the value of the combo-weapons setting (2), the absence of the combo pickup from all 59 worlds; **medium** for everything read from `Sam2Game.dll` (the index table's file names and its deathmatch rule, the default weapon sets, the strength order, the receive rules, the co-op stay rule, the combo pickup and the combo mode, the dual-wielding switch), because each rests on **one reading** of the decompiled functions plus a read of the tables and **nothing was run**; **low** for the items marked low and for "Open questions".

Prerequisites: `weapon-classes.md` (section 8 describes the pickup classes), `item-classes-ammo-weapons.md` (section 4: the 14 pickup classes with their quantities, models and respawn times), `item-classes.md` (the pickup protocol and the co-op stay options), `players-session-classes.md` (the inventory, the enumerations, the ammunition tables), `weapon-view.md` (the two-weapon rules).

## Summary

1. **Which files a weapon uses is a fixed table of the binary**: index 0 to 13, 15, 16 each name `EntityParams/<Name>Weapon.ep` and `EntityParams/<Name>WeaponRendering.ep` (section 1); the **deathmatch variant** is chosen **when the weapon entity looks its file up** by replacing `.ep` with `_DM.ep` in a deathmatch session; the **pickup** always links the single player file (only the weapon entity uses the variant). Indices 14 (combo) and -1 have no files.
2. **A player starts with** the circular saw, the zap gun and the Colt (the saw is left out in a deathmatch). **Only the Uzi and the Colt can be doubled** (held in both hands) unless the combo weapons are on.
3. **Receiving a weapon** only sets its bit, never grants anything else; **the pickup grants the ammunition**: the pickup's quantity, **multiplied by the session's enemy multiplier** and clamped to the cap of the ammunition kind (section 3).
4. **The weapon pickup is needed** while the player lacks the weapon **or** his ammunition of its kind is below the cap, so **a pickup keeps refilling ammunition until the cap**. Co-op: the pickup is consumed by the first player only when the session option `sgp_bWeaponStaysInCoop` is off (default **on**: the weapon stays) (section 4).
5. **Auto-change:** a received weapon is selected at once when the player's setting is "on" (1), or when it is stronger than the weapon in the right hand under the fixed **strength order** of section 2 when the setting is "only if stronger" (2).
6. **The combo weapons** (`ggp_cwuComboWeaponsUsing`, `CWU_*`) are **enabled in the data (value 2) but switched off by the default session option `sgp_bUseComboWeapons` (0)**; no world places the combo pickup (0 of 59). With the option on, a **dual-wielding switch** lets the player hold two different weapons and fire them freely (section 5).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Exported functions read, by name: `samGetWeaponParamsPath`, `samGetWeaponRenderParamsPath`, `samGetAmmoForWeapon`, `samCreateWeaponItemProperties`, `samGetDefaultWeapons`, `samGetDefaultDoubleWeaponFlags`, `samGetWeaponStrengthIndex`, `samGetAmmoSpending`; `CWeaponItemEntity::OnCreate`, `IsNeeded`, `OnPicked`, `OnlyFirstPlayerPicks`; `CComboWeaponsItemEntity::OnCreate`, `OnPicked`; `CBaseWeaponEntity::ObtainWeaponParams`; `CPlayerInventory::ReceiveWeapon`, `ReceiveAmmo`, `RemoveWeapon`, `ClearWeapons`, `ResetAllToDefault`, `IsWeaponInInventory`, `IsWeaponDoubled`, `CanBeUsedForFiring`, `DrainAmmoFromWeapon`; `CPlayerPuppetEntity::ReceiveWeapon`, `ReceiveAmmo`, `SetNewWeapon`, `SetNewWeaponToPlayerHand`, `ToggleDualWielding`, `IsDualWielding`, `IsComboWeaponsModeActive`, `GetComboWeaponsModeDuration`, `StartComboWeaponsMode`, `CanFireFromWeaponNow`; `CGameInfoEntity::IsComboWeaponsEnabled`. The strings and tables were read from the data section. The path function names every file as a string; the full path in the data adds the database folder (`Content/SeriousSam2/Databases/`).
- **Data.** `tools/re/weapon_pickups_verify.py [DATA_DIR]` (about 2 minutes) prints the table of section 1 with the existence and class of each file, the leftover files and the combo facts. The quantities and models of the placed pickups are `item_classes_verify.py` (section 4 of `item-classes-ammo-weapons.md`).
- **Not checked:** nothing was run; the weapon selection menu, the favourite and next and previous weapon search (T01-114), the HUD list, the saved game carry-over of the doubled flags (`save-game.md`), the left-hand weapon choice UI.

## Layout / rules

### 1. The index to file table (**medium** for the names, **high** that each file exists in the data)

| `WeaponIndex` | Name | Parameter file | Class in the data |
|---|---|---|---|
| 0 | circular saw | `EntityParams/CircularSawWeapon.ep` | `SCircularSawWeaponParams` |
| 1 | zap gun | `ZapGunWeapon.ep` | `SZapGunWeaponParams` |
| 2 | auto shotgun | `AutoShotgunWeapon.ep` | `SAutoShotgunWeaponParams` |
| 3 | double shotgun | `DoubleShotgunWeapon.ep` | `SDoubleShotgunWeaponParams` |
| 4 | Uzi | `UziWeapon.ep` | `SUziWeaponParams` |
| 5 | minigun | `MiniGunWeapon.ep` | `SMiniGunWeaponParams` |
| 6 | rocket launcher | `RocketLauncherWeapon.ep` | `SRocketLauncherWeaponParams` |
| 7 | grenade launcher | `GrenadeLauncherWeapon.ep` | `SGrenadeLauncherWeaponParams` |
| 8 | plasma rifle | `PlasmaRifleWeapon.ep` | `SPlasmaRifleWeaponParams` |
| 9 | Klodovik gun | `KlodovikGunWeapon.ep` | `SKlodovikGunWeaponParams` |
| 10 | cannon | `CannonWeapon.ep` | `SCannonWeaponParams` |
| 11 | Serious Bomb | `SeriousBombWeapon.ep` | `SSeriousBombWeaponParams` |
| 12 | Colt | `ColtWeapon.ep` | `SColtWeaponParams` |
| 13 | sniper | `SniperWeapon.ep` | `SSniperWeaponParams` |
| 14 | combo weapons | none (empty string) | |
| 15 | beam gun | `BeamGunWeapon.ep` | `SBeamGunWeaponParams` |
| 16 | flamer | `FlamerWeapon.ep` | `CFlamerWeaponParams` |

The **rendering file** of each is `EntityParams/<same name>WeaponRendering.ep` (16 files, class `S<Name>WeaponRenderingParams`, the flamer's `CFlamerWeaponRenderingParams`). Any other index (-1, 14, 17 and above) gives the **empty string** and the pickup then links nothing. **Deathmatch:** `ObtainWeaponParams` of the weapon entity takes the path of the index and, **in a deathmatch session** (`samIsDeathmatch`), replaces `.ep` by `_DM.ep` (`ColtWeapon_DM.ep` and so on, 16 files, all present); the **rendering file has no variant**, and the pickup's own link (`en_pbwpWeaponParams`) is the single player file in every session. Files in the folder that no index names: `CircularSawBladesWeapon.ep` and its `_DM.ep` (the blades, no index of their own) and the leftover `BeamGunWeapon - Copy.ep` (`weapon-classes.md`).

### 2. The weapon tables of the inventory (**medium**)

- **Default weapons** (`samGetDefaultWeapons`; a new player, `ResetAllToDefault` and `ClearWeapons`): **circular saw, zap gun and Colt** (bits 0, 1 and 12: flag value 4,099), **without the saw in a deathmatch** (4,098).
- **Default doubled weapons** (`samGetDefaultDoubleWeaponFlags`): **Uzi and Colt** (bits 4 and 12: 4,112). The inventory keeps a second flag set for them; **no function sets a doubled flag** except these resets, and `RemoveWeapon` clears it with the weapon bit. So with the combo option off a player can hold **two Uzis or two Colts** (the pickup of a second one does not matter: the flag is there from the start) and no other pair.
- **Strength order** (`samGetWeaponStrengthIndex` is the position in a table of the 17 values, weakest first): saw, Colt, zap gun, auto shotgun, double shotgun, Uzi, minigun, rocket launcher, grenade launcher, plasma rifle, beam gun, sniper, Klodovik gun, cannon, Serious Bomb, combo weapons (14), flamer. An index not in the table has strength -1. The ranks are 0 to 16 in this order.
- **Ammunition kind and cost per shot**: the tables of `players-session-classes.md` section 7.3 (kind: saw none, zap gun none, shotguns shells, Uzi and minigun bullets, rocket rockets, grenade grenades, plasma and beam gun plasma, Klodovik Klodovik, cannon cannon balls, bomb serious bomb, Colt none, sniper sniper bullets, flamer napalm; cost per shot 1 except the double shotgun 2). **With the session option `sgp_bInfiniteAmmo` the kind of every weapon is "none"** (`samGetAmmoForWeapon`), which also makes every weapon usable and every pickup of weapon ammunition grant nothing.

### 3. Receiving a weapon and its ammunition (**medium**)

- **`CPlayerPuppetEntity::ReceiveWeapon(index)`** runs **on the host/local machine only** (not on a client). The Serious Bomb is **not received** while serious bombs are disabled (`samIsSeriousBombsDisabled`); `-1` is ignored. Otherwise the bit of the index is set in the inventory (the inventory function additionally **clears** the bomb bit and its doubled bit when bombs are disabled).
- **Auto-change** (`en_acwAutoChangeWeapon`, `AutoChangeWeapon`): after receiving, `SetNewWeapon(index, ...)` is called when the setting is **1 (`ACW_ON`)**, or when it is **2 (`ACW_ONLY_IF_STRONGER`)** and the weapon in the **right hand** has a **lower strength rank** than the received one (**or there is no weapon in the right hand**); with **0 (`ACW_OFF`)** never. (Only the right hand is compared.)
- **`ReceiveAmmo(kind, quantity)`** (inventory, via the player; host only): a kind outside 0 to 11 is ignored; **new count = enemy multiplier × quantity + old count**, **clamped to 0 .. the cap of the kind**, where the enemy multiplier is `CGameInfoEntity::GetEnemyMultiplier` (**1 when there is no game info**; its rule is `players-session-classes.md` section 7.3) and the cap is `samGetMaxAmmoQuantity` (the cap table of `players-session-classes.md` section 7.3). So **a pickup's quantity is scaled by the multiplier**, in co-op too; test vector 1.
- The pickup's `OnPicked` calls the base pickup (score, hiding, respawn, `item-classes.md`), then `ReceiveWeapon(index)`, then `ReceiveAmmo(kind of the index, the pickup quantity)`; the quantity is `en_slAmmoQuantity` of the pickup (`item-classes-ammo-weapons.md`: zap gun and Colt 0, because they need no ammunition; Klodovik 1; and so on).
- **Drops:** `DropWeapon` makes a pickup only for a weapon with an ammunition kind and gives it the player's **current** count of that kind; unchanged from `item-classes-ammo-weapons.md` section 4.

### 4. The pickup test and the co-op stay rule (**medium**)

- **`IsNeeded(player)`**: true when the player does **not** have the weapon; otherwise true while his count of the weapon's ammunition kind is **below its cap** (`GetMaxAmmoQuantity`). For a weapon without an ammunition kind (zap gun, Colt, saw; every weapon under infinite ammunition) the kind is "none": the cap is 0 and the count is read as not below it, so a **second pickup of such a weapon is not needed** (**medium-low**: the count of kind "none" was not read in detail).
- **`OnlyFirstPlayerPicks`**: in a **co-op** game (game type 2) **true when `sgp_bWeaponStaysInCoop` is off**; in every other game type the base item's rule (`item-classes.md`). With the default (**on**) every co-op player may take the same pickup, which stays for the others (hidden per player, `item-classes.md`).
- With the default session options the **combination** is: in co-op each player takes each weapon pickup once as a new weapon, and again whenever his ammunition for it is below the cap; the pickup never disappears.

### 5. The combo weapons (**medium**; the mode's expiry **medium-low**)

**Settings.** `ggp_cwuComboWeaponsUsing` (`ComboWeaponsUsing`) is **2 (`CWU_ALLOW_CUSTOM`)** in the data. The game info answers `IsComboWeaponsEnabled` true only when **all three** hold: the setting is 2, the session option **`sgp_bUseComboWeapons`** is on (**default 0**, `helper-classes-params.md`) and the game type is **single player or co-op** (never deathmatch). So **in a default game the combo weapons are off**.

**The combo pickup (`CComboWeaponsItemEntity`).** Binary only; **no world places it** (0 of 59). Its five members (the left-hand weapon, the ammunition for it, the right-hand weapon, the ammunition for it, an auto-off time) are copied at creation and **only the two child models named after the two weapons are shown**. `OnPicked`, **only when the setting is not `CWU_NEVER` (0)** (otherwise nothing at all happens, not even the base pickup):
1. stores the two weapon indices as the player's **last combo weapons**;
2. gives the player each of the two weapons **if missing** (`ReceiveWeapon` of the inventory, so no auto-change);
3. gives the ammunition: `ReceiveAmmo(kind of the left weapon, left quantity)` and the same for the right (the enemy multiplier applies, 3);
4. **when the setting is `CWU_ITEM_CONTROLLED` (1)**: `StartComboWeaponsMode(auto-off time)`: a negative time stores -1 ("no limit"), otherwise it stores the start time (now) and the duration; with the retail setting 2 this step is **skipped**;
5. the base weapon pickup (score, hiding, `ReceiveWeapon(14)`, no ammunition because kind "none").
`IsNeeded` is the weapon rule with index 14: the player's ammunition kind is "none", so the pickup is **always needed** once taken once or not (**medium-low**).

**The mode.** `IsComboWeaponsModeActive`: **false** when the setting is 0; **true** when the setting is 2 and combo is enabled; otherwise (setting 1) **true while the time since `StartComboWeaponsMode` is below the stored duration, or always when the duration is negative**, false when the duration has run out (**medium-low**: the final return of the function was lost by the decompiler). `GetComboWeaponsModeDuration` is the time since the start.

**The dual-wielding switch** (`ToggleDualWielding`, `IsDualWielding`; the player's switch, the member the other pages call the dual flag): `IsDualWielding` is true when both hands hold a weapon **and** the switch is on. The toggle is **ignored** while the player is riding or in a second state that was not identified (a handle of the puppet, probably carrying an object), when the right-hand weapon answers a weapon query that was not identified (probably "is zooming"), when the right-hand weapon is the Serious Bomb, and when either weapon is **not idle or boring**. Turning it **on** with **no left-hand weapon yet** needs the **currently selected weapon to be a doubled one** (Uzi, Colt), otherwise nothing happens; an existing left weapon is kept or replaced by the wanted one after it is put down; turning it **off** puts the left weapon down (**medium-low**: a long function read for its conditions only). While the switch is on **and combo weapons are enabled**, `CanFireFromWeaponNow` always allows firing (`weapon-view.md` section 5 rule 1) so that two **different** weapons fire independently; with the option off, only equal weapons can be held in two hands and they alternate (`bwp_fDoubleWeaponFireDelay`). The exact selection of the left weapon (the choosing mode, the buttons) is T01-114.

### 6. Binary-only pieces, for the implementer's orientation

- `samCreateWeaponItemProperties` has cases for 1 to 13, 15 and 16 only (`item-classes-ammo-weapons.md`).
- The combo weapon has **no weapon entity**: dual wielding holds two ordinary entities (`weapon-classes.md` section 6.2).

## Invariants

1. All 16 weapon files, 16 deathmatch variants and 16 rendering files named by the table exist in the data (48 of 48); the beam gun's leftover copy and the blades' files are the only others.
2. A weapon index without a file name (-1, 14, 17 and above) gives the empty string for both files.
3. The strength order is a permutation of the 17 indices (0 to 16, each once).
4. The default weapon set has 3 weapons (2 in deathmatch) and the default doubled set 2 (Uzi, Colt).
5. No placed object of the combo pickup class exists in the 59 worlds; the data setting is 2.
6. Ammunition after a pickup is within 0 and the cap of the kind.

## Open questions

1. **The weapon selection rules** (favourite, next, previous, last, the choosing of the left-hand weapon, `ToggleToWeapon`) and the left hand's use of the fire buttons: **T01-114.**
2. **Whether the doubled flags carry over between chapters or are set by scripts** (the saved game stores the inventory, `save-game.md`): not read here. **T01-225.**
3. **The unread end of `IsComboWeaponsModeActive`** and the owner's use of the mode in `OnStep` (the end of an item-controlled mode, the HUD): **T01-225.**
4. **Run-time checks** of the pickup amounts, the enemy multiplier at a pickup in co-op and the dual-wield alternation: **T01-124** and **T01-163.**
5. The cheat "give all weapons" set (49,151) that omits the combo and the flamer bits (`weapon-classes.md` open question 9) was not read: **T01-114.**

## Test vectors

Derived from the data and the rules above; **not** run on the original.

1. **Ammunition from a pickup.** Single player (enemy multiplier 1): an auto shotgun pickup (quantity 10) to a player with 195 shells and the cap 200: `10 × 1 + 195 = 205`, clamped to **200**. In a game where the multiplier is 2: `2 × 10 + 150 = 170` (not clamped).
2. **Needed.** A player with the auto shotgun and 200 shells (cap 200): the pickup is **not needed**; with 199 shells it is needed. A player without the weapon: always needed.
3. **Auto-change.** Setting 2, right hand holds the Colt (rank 1): receiving the Uzi (rank 5) switches; receiving the saw (rank 0) does not; right hand empty: switches to anything. Setting 1: always switches. Setting 0: never.
4. **Files.** Index 12, single player: `EntityParams/ColtWeapon.ep`, rendering `EntityParams/ColtWeaponRendering.ep`; in a deathmatch session the **weapon** loads `EntityParams/ColtWeapon_DM.ep`; the pickup still links `ColtWeapon.ep`. Index 14: both empty.
5. **New player.** Single player: weapons {saw, zap gun, Colt}, doubled {Uzi, Colt}. Deathmatch: {zap gun, Colt}, doubled {Uzi, Colt}.
6. **Combo pickup, setting 2 (retail) with the option off:** the pickup would store the two weapons, give both weapons and ammunition and the base pickup, but starts no timed mode (the data setting is not 1); with the setting 0: nothing at all.
