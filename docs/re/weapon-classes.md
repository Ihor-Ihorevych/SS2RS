# Weapon classes of `Sam2Game.dll` (T01-106, part of T01-026)

- **Roadmap task:** T01-106 (part 5 of the class inventory T01-026; the other parts are T01-102 to T01-113)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/weapon_classes_verify.py` reproduces from the data: which of the 83 classes are saved (44), under which name, their base class, member counts and file counts, and the **member-order rule** of section 4 (all 14 data classes that the binary also declares agree on member count, on the position of every member the data names, and on the data types); **high** for three member meanings that two independent readings confirm (`ctFiresToReload`, `LaunchVelocityMin` and `LaunchVelocityMax`, section 4.1); **medium** for everything else read from the binary (exported names, parents, the member tables with their names and flags, the enumerations, the overridden virtual hooks and the rules of section 9), because each rests on one reading and **nothing was run**; **low** for what a method does when it is described only from its name (marked "(name)") and for the items in "Open questions".

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `ep.md` and `ep-schema.md` (the `.ep` files), `weapons.md` and `weapons-tables.md` (the parameter values of the 17 weapons), `players-session-classes.md` (the player, the inventory, the weapon and ammunition enumerations, the cost per shot), `replication.md` (net behaviour and replicated members), `player-rules.md`.

## Summary

`Sam2Game.dll` exports **83 classes** (780 exported functions, 83 `md_pdtDataType` data symbols; 203 named methods besides the standard set) that make up the weapons. They are **the weapon entities, their parameter files and the weapon pickups**. Of the 83, **44 are saved in the data** and **39 exist only in the binary**: all 17 weapon entities, all 17 weapon properties classes, the base entity and the base properties class, the developer tool `CAdjustWeaponMode` and the combo-weapons pickup (entity and properties).

What a reader needs to know first:

1. **A weapon is three layers plus a rendering layer.** For each of the **17 weapons** there is an **entity** (`C<Weapon>WeaponEntity`, the live object a player carries; binary only), a **properties** class (`C<Weapon>WeaponProperties`; binary only, empty), a **parameters** class (`C<Weapon>WeaponParams`, the `.ep` file: in the data) and a **rendering parameters** class (`C<Weapon>WeaponRenderingParams`, first-person placement: in the data; 16 weapons, the circular saw's blades have none). That makes 16 weapons with a weapon index plus the **blades** of the circular saw (`CCircularSawBladesWeapon*`: entity and parameters, no rendering parameters, no weapon index of its own) = **17 weapon classes**; weapon index 14 is the combo weapons and has no class.
2. **No weapon entity is ever in a world file.** Weapon entities are created at run time by the player puppet when a weapon is received (`players-session-classes.md`), exactly as the player entities are. Only the **pickups** (`SWeaponItemEntity` and its children) are placed in worlds (46 worlds).
3. **The 22 unnamed base members of the weapon files get their names here.** The data stores most members of `SBaseWeaponParams`, `SBaseScatterWeaponParams`, `SBaseWeaponRenderingParams` and of five weapon classes with a bare number, because the data's member names are partly numbers. The binary declares the **same members with names**, in the **same order**, and the order rule of section 4 (checked on 14 classes) assigns every number its name. This answers most of `weapons.md`'s "unnamed" list and part of T01-075: for example (confidence per member in section 4.1) `b.30` and `b.31` are the **minimum and maximum recoil duration**, `b.5` is the **number of shots before a reload** (6 for the Colt, 0 for every other weapon), `b.13` is the **charging time**, `b.15` and `b.16` the **launch velocity at no charge and at full charge**.
4. **The weapon entity does not shoot by itself.** The hitscan weapons (Colt, Uzi, minigun, both shotguns, sniper) own a **shooter** entity that they hand a bullet description; the projectile weapons ask the **owner puppet** to launch the projectile named by `bwp_pgppProjectileParams`. The base class's fire function only pays the ammunition and returns whether the shot may happen, and it returns "no" unless the owner is a player puppet (section 9). The shooter and the projectile are T01-107.
5. **Enumerations.** The binary declares names and values of the weapon state machine (10 states), the bullet types, the charging flags, the minigun spin and fire types, the player hand and the damage-inflictor weapon (section 3).

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were listed and the 932 exported classes grouped by name (the split of T01-026). The 83 classes of this family are all those whose name matches the weapon pattern, the pickups of the cannon and its balls, the combo-weapons pickup, `CMountedTool`, `CThrowDescription` and `CAdjustWeaponMode`. Their exported functions total **780**, which equals the survey of T01-026. All 83 names are also strings of `InSamnity2Game.dll` (the mod DLL, T13) and of no other module.
- **Class registrations and member tables.** The DLL carries a registration record for every class (parent name, a caption for some, the data name) and static member tables (name, type reference, byte offset, flags). 229 member tables were scanned, 27 of them (with members) are of this family; the registration records of the classes without members were read for the rest. Byte offsets and sizes were read but are **not recorded** (`conventions.md`). Parents come from the registrations (the parent's C name or, for the base classes, the `S` data name) and, for entity classes, were checked against the virtual function tables.
- **Virtual function tables.** The table of each of the 23 classes whose virtual functions were compared (the 18 weapon entities, the four pickup entities and `CAdjustWeaponMode`) was compared slot by slot with its parent's: `CBaseWeaponEntity` has 160 slots against 102 of `CBaseEntity` (13 overridden, 58 new); every weapon entity has the 160 slots of its parent or 161 to 164 with new ones (section 6). Identical small functions are shared by the linker, so a slot can show another class's name (for example `CColtWeaponEntity::SetOwner` stands for the shared owner functions of six weapons); such slots are not used for meaning.
- **Rules (section 9)** come from reading **small exported functions in full** (getters, state tests, the fire functions of each weapon, the pickup functions). Each is one reading (**medium**); where the data or a second function agrees this is said. No function was read as a whole state machine.
- **Enumerations** were read as (value, name, caption) triples in the DLL's data and checked against the counts of the data (`WeaponIndex` has 17 values + none; `AmmoIndex` 12 + none) and against `players-session-classes.md` (same lists).
- **Data.** `tools/re/weapon_classes_verify.py` reads the type tables of all 8,063 format-8 files and the 17 + 17 + 16 weapon files (about 4 minutes) and prints the counts of sections 2 and 4 and the tables of 4.1 and 4.2.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run; no weapon's per-frame behaviour (the state machine's step function, recoil, spin, zoom, the beam and saw damage functions) was read.

## Layout / rules

### 1. Names, layers and the data names

| Rule | Statement | Evidence | Confidence |
|---|---|---|---|
| **C name exported, S name saved** | `CColtWeaponParams` is exported; the data stores `SColtWeaponParams`. Of the 44 saved classes **40 appear as the `S` twin** and **4 under the exported `C` name**: `CFlamerWeaponParams`, `CFlamerWeaponRenderingParams`, `CMountedTool`, `CThrowDescription` | script, section 1 | high for the counts, medium that a twin is the same class (as `players-session-classes.md` section 1) |
| **Entity, properties, parameters, rendering parameters** | `Entity` = the live object, `Properties` = what a world would store for it (the 17 properties classes have **no members** of their own and only the standard symbols; `SBaseWeaponProperties` has two), `Params` = a shared `.ep` resource (the weapon), `RenderingParams` = a second `.ep` (first-person placement) | exports, tables, script | medium |
| **Hierarchy** | `CBaseWeaponEntity` ← `CBaseEntity`; every weapon entity ← `CBaseWeaponEntity`; `CBaseWeaponParams` ← `CEntityParams`; `CBaseScatterWeaponParams` ← `CBaseWeaponParams`; bullet weapons' params ← `CBaseScatterWeaponParams`, all other weapons' params ← `CBaseWeaponParams`; weapon properties ← `CBaseWeaponProperties` ← `CBaseProperties`; rendering params ← `CBaseWeaponRenderingParams` ← `CEntityParams`; pickups: `CWeaponItemEntity` ← `CBaseItemEntity`, `CCannonItemEntity` and `CComboWeaponsItemEntity` ← `CWeaponItemEntity`, `CCannonBallsItemEntity` ← `CAmmoItemEntity` | registrations (all), vtables (entities), script for the data's `base` column | high for the data classes (script), medium for the binary-only ones |
| **Standard symbols** | Classes have the default and copy constructors (two `ctor` exports for classes with a copy form), the destructor (most), the assignment and the stream-read operators, the vftable when it has virtual functions and `mdGetDataType`. 61 of the 83 classes export exactly 7 symbols (59 of them only this standard set: the properties, parameters and rendering classes that add nothing, the blades entity and four pickup properties classes; the other two are the records `CMountedTool` and `CThrowDescription`) | export table | medium |
| **Defining module** | all 83 in `Sam2Game.dll` (the names are strings in no other module except the mod DLL) | string search of every binary in the install's `Bin` folder | high |

### 2. The 83 classes

"Saved as" is the data's type name (script section 1). "Members" is the number of members the data's type table gives it. "Files" is the number of files whose type table has it. `S`/`C` says which name the data uses. "Symbols (methods)" is exported symbols (named methods other than the standard set and `mdGetDataType`); the methods sum to 203.

| Group | Classes | Parent | Saved as | Members / files | Symbols (methods) |
|---|---|---|---|---|---|
| base entity | `CBaseWeaponEntity` | `CBaseEntity` | not saved | binary table 34 members | 78 (71) |
| base properties | `CBaseWeaponProperties` | `CBaseProperties` | not saved | binary table 2 members | 7 (0) |
| base params | `CBaseWeaponParams` | `CEntityParams` | `SBaseWeaponParams` | 22 / 35 `.ep` | 7 (0) |
| scatter params | `CBaseScatterWeaponParams` | `CBaseWeaponParams` | `SBaseScatterWeaponParams` | 9 / 12 `.ep` | 7 (0) |
| base rendering | `CBaseWeaponRenderingParams` | `CEntityParams` | `SBaseWeaponRenderingParams` | 8 / 16 `.ep` | 7 (0) |
| 17 weapon entities | `C<Weapon>WeaponEntity` | `CBaseWeaponEntity` | not saved | section 6 | 7 to 26 each, 112 named methods in all (section 6.2) |
| 17 weapon properties | `C<Weapon>WeaponProperties` | `CBaseWeaponProperties` | not saved | none | 7 (0) each |
| 17 weapon params | `C<Weapon>WeaponParams` | `CBaseScatterWeaponParams` (the 6 bullet weapons) or `CBaseWeaponParams` (the other 11) | `S<Weapon>WeaponParams`, `CFlamerWeaponParams` | 0 to 12 / 2 `.ep` each (3 for the beam gun: single player, deathmatch and a leftover file named `BeamGunWeapon - Copy.ep`) | 7 (0) each |
| 16 rendering params | `C<Weapon>WeaponRenderingParams` (not the blades) | `CBaseWeaponRenderingParams` | `S<Weapon>WeaponRenderingParams`, `CFlamerWeaponRenderingParams` | 0 / 1 `.ep` each | 7 (0) each |
| pickups | `CWeaponItemEntity`, `CWeaponItemProperties` | `CBaseItemEntity`, `CBaseItemProperties` | `SWeaponItemEntity` (4 / 46 `.wld`), `SWeaponItemProperties` (0 / 48: 46 `.wld`, 2 `.ctl`) | | 14 (7), 7 (0) |
| | `CCannonItemEntity`, `CCannonItemProperties` | `CWeaponItemEntity`, `CWeaponItemProperties` | `SCannonItemEntity`, `SCannonItemProperties` | 0 / 15 `.wld` each | 8 (1), 7 (0) |
| | `CCannonBallsItemEntity`, `CCannonBallsItemProperties` | `CAmmoItemEntity`, `CAmmoItemProperties` | `SCannonBallsItemEntity`, `SCannonBallsItemProperties` | 0 / 19 `.wld` each | 9 (2), 7 (0) |
| | `CComboWeaponsItemEntity`, `CComboWeaponsItemProperties` | `CWeaponItemEntity`, `CWeaponItemProperties` | not saved (5 members each in the binary) | | 10 (3), 7 (0) |
| tools | `CAdjustWeaponMode` | none found | not saved | | 10 (4) |
| shared records | `CMountedTool`, `CThrowDescription` | none | same names | 2 / 9 `.ep`, 6 / 15 `.ep` | 7 (1), 7 (2) |

Counts: 5 base classes + 17 × 4 − 1 (the blades have no rendering class) + 8 pickup classes + 3 others = **83**. In the data **44** = 3 base classes (`SBaseWeaponParams`, `SBaseScatterWeaponParams`, `SBaseWeaponRenderingParams`) + 17 weapon params + 16 rendering params + 6 pickup classes (`SWeaponItemEntity`, `SWeaponItemProperties`, the cannon item entity and properties, the cannon-balls item entity and properties) + `CMountedTool` + `CThrowDescription`. **High** (script). The **39** binary-only classes: 17 entities, 17 properties, the base entity, the base properties, the combo pickup's entity and properties, `CAdjustWeaponMode`.

The **other weapon pickups** (`C<Weapon>ItemEntity` for the Colt, Uzi, ..., each a child of `CWeaponItemEntity`, in 12 to 23 worlds each in the data) and the ammunition pickups belong to T01-108; only the cannon's pickups are here because the survey put them in this family.

### 3. Enumerations declared by the binary

Value, name, caption as the DLL gives them. These are binary identifiers and are quoted as they are. **Medium** (one table each, read twice: strings and a table parse).

| Enumeration | Values |
|---|---|
| `WeaponState` (member `en_wsState`) | 1 `WS_IDLE`, 2 `WS_PUTTING_DOWN`, 3 `WS_BRINGING_UP`, 4 `WS_SPIN_FIRING`, 5 `WS_CHARGING`, 6 `WS_CANT_FIRE`, 7 `WS_BORING`, 8 `WS_RECOIL`, 9 `WS_RELOAD`, 10 `WS_WAIT_FUSE_TO_BURN` (captions: Idle, Putting down, Bringing up, Spin firing, Charging, Can't fire, Boring, Recoil, Reload, Wait fuse to burn) |
| `WeaponIndex` (the weapon number) | -1 none, 0 circular saw, 1 zap gun, 2 auto shotgun, 3 double shotgun, 4 Uzi, 5 minigun, 6 rocket launcher, 7 grenade launcher, 8 plasma rifle, 9 Klodovik gun, 10 cannon, 11 Serious Bomb, 12 Colt, 13 sniper, 14 combo weapons, 15 beam gun, 16 flamer (as in `players-session-classes.md`) |
| `WeaponFlags` (bit per weapon) | bit `1 << WeaponIndex` for the same 17 weapons, `WF_CIRCULAR_SAW` 1 to `WF_FLAMER` 65,536 (the cheat set 49,151 of `player-rules.md` has bits 0 to 13 and 15 set: it leaves out the combo weapons, bit 14, and the flamer, bit 16; see Open questions) |
| `DamageInflictorWeapon` (who gets the credit for a hit; member `en_diwDamageInflictorWeapon`) | -1 none, 0 to 13 equal to `WeaponIndex` 0 to 13 (13 = sniper), 14 hand grenade, 15 turret, 16 hovercraft, 17 aircraft, 18 beam gun, 19 flamer |
| `BulletType` (member `bswp_btBulletType`) | 0 `BT_PHYSICAL` (a flying bullet), 1 `BT_RAY_STATIC_TRAIL` (immediate, static trail), 2 `BT_RAY_MOVING_TRAIL` (immediate, moving trail), 3 `BT_RAY_NO_TRAIL` (immediate, no trail) |
| `WeaponChargingFlags` (member `en_ulWeaponChargingType`) | 1 `WCF_CHARGE`, 2 `WCF_SPINUP` |
| `PlayerHand` | 0 `PH_LEFT_HAND`, 1 `PH_RIGHT_HAND` |
| `MGFireType` / `MGSpinType` (minigun sound states) | fire: 0 none, 1 `MGFT_FIRE`, 2 `MGFT_CLICK`; spin: 0 none, 1 `MGST_SPINUP`, 2 `MGST_SPIN`, 3 `MGST_SPINDOWN` |
| `ComboWeaponsUsing` (`ggp_cwuComboWeaponsUsing`) | 0 `CWU_NEVER`, 1 `CWU_ITEM_CONTROLLED` ("Controlled by combo item"), 2 `CWU_ALLOW_CUSTOM` ("Allow custom combination"); the data has 2 |
| `AutoChangeWeapon` (`en_acwAutoChangeWeapon`) | 0 `ACW_OFF`, 1 `ACW_ON`, 2 `ACW_ONLY_IF_STRONGER` |

Cross-check (**high**): the data's bullet types in the weapon files are 1 (Colt, Uzi, sniper) and 2 (both shotguns, minigun) (script section 3), values that exist in the enumeration; `en_wiWeaponIndex` of the pickups ranges 2 to 15 in the data (`wld-entity-schema.md`), inside the 17 values.

### 4. Parameter classes: the member names

#### 4.0 The member-order rule

The type table that each data file carries lists the members of a class in an order and names some of them (`aswp_ctBullets`) while others have only a number (`30`). The binary declares for the same class a member table in the same order, with a name for every member. The script compares the two for the 14 classes of section 4 (`BIN` in the script holds the binary's names only) and finds, for **all 14**: (a) the same number of members, (b) every member that the data names is at the position of the same name in the binary's table, (c) the data type of every member fits the Hungarian prefix of the binary's name (a `pmc` name is a `Ptr<CModelConfiguration>`, `ct` an integer count, `f` a `FLOAT`, `v` a vector, `id` an `IDENT`, `pct` a `Ptr<CCharacterTool>`, `pgpp` a `Ptr<CGenericProjectileParams>`). **Rule: the number-only member at position *n* of the data class is the member the binary declares at position *n*.** Confidence: **high** for the rule on these 14 classes (script); **medium** for each assignment's meaning where it rests on the name alone. The numbers are only labels: they do not equal the position (`SBaseWeaponParams.24` is position 0). `CFlamerWeaponParams` is the exception: the data has one definition with no member and one with a member `4` (the flamer's deathmatch file), while no member table of that class was found in the binary; the member `4` is therefore **not read by this build** as far as can be said (low), which matches `weapons.md` (a fire effect named in the deathmatch file only).

#### 4.1 `SBaseWeaponParams` (all 17 weapon files, 22 members)

"Data" is the member's number (`b.N` of `weapons.md`) and the values over the 17 single-player files. **Meaning** is the binary's name with what is established; the unit is seconds unless stated.

| Pos | Data | Binary name | Meaning | Values in the data | Confidence |
|---|---|---|---|---|---|
| 0 | `b.24` | `bwp_pmcModel` | first-person model | 17 `.mdl` | high (type) |
| 1 | `b.35` | `bwp_pctLeftHandTool` | tool (`.ctl`) played for the left hand: `GetWeaponTool(PH_LEFT_HAND)` returns this member | 17 | medium (name; getter read) |
| 2 | `b.36` | `bwp_pctRightHandTool` | tool for the right hand | 17 | medium |
| 3 | `b.5` | `bwp_ctFiresToReload` | **shots before a reload**: `ShouldReload` is true when this is positive and the number of shots fired since the last reload has reached it | Colt 6, all others 0 | **high** (code read + data) |
| 4 | `b.13` | `bwp_fChargingTime` | seconds of charging until full power: the charging weapons (grenade launcher, cannon, zap gun) | grenade launcher 0.5, cannon 1.25, zap gun 0.8, others 0 | medium |
| 5 | `b.33` | `bwp_fChargingAutoFireTime` | seconds after which a held charge fires by itself | zap gun 0.3, others 0 | medium (name) |
| 6 | `b.30` | `bwp_fMinRecoilDuration` | the **recoil duration**, which is the **period between shots**; `GetRecoilDuration` interpolates between this and the next member by the fire power, and **0 means "as long as the recoil animation"** | 0 (sniper sp, saw, bomb) to 1.3 (`weapons.md` table) | medium (one function read; the sniper's 0 fits) |
| 7 | `b.31` | `bwp_fMaxRecoilDuration` | the recoil duration at full fire power | 0 for most; cannon 0.75, grenade launcher 0.5, zap gun 0.5, plasma rifle 0.15 | medium |
| 8 | `b.32` | `bwp_fReloadDuration` | when positive, the reload animation is rescaled to this duration (the direction of the factor is not read); every weapon with a reload has 0 (the Colt) | flamer 0.05, rocket launcher 1, others 0 | medium |
| 9 | `b.7` | `bwp_fOffsetX` | view kick / sway amplitude along X (name; **meaning of the group 9 to 14 unread**) | 0.005 or 0.01 | low |
| 10 | `b.9` | `bwp_fOffsetY` | the same along Y | 0.005 or 0.01 | low |
| 11 | `b.11` | `bwp_fOffsetZ` | the same along Z | 0.1 or 0.4 | low |
| 12 | `b.8` | `bwp_fFrequencyX` | frequency of the X motion | 5 or 10 | low |
| 13 | `b.10` | `bwp_fFrequencyY` | frequency of the Y motion | 6 or 8 | low |
| 14 | `b.12` | `bwp_fOffsetZExponent` | exponent of the Z motion | 2 or 3 | low |
| 15 | `b.15` | `bwp_fLaunchVelocityMin` | speed of the launched projectile at no charge: the grenade launcher, cannon and zap gun launch with `min + (max - min) × fire power` | grenade launcher 10, cannon 40, zap gun 40, others 0 | **high** (the three weapons' fire functions read; data nonzero for exactly those three) |
| 16 | `b.16` | `bwp_fLaunchVelocityMax` | the speed at full charge | grenade launcher 130, cannon 140, zap gun 25, others 0 | **high** (same) |
| 17 | `b.34` | `bwp_fDoubleWeaponFireDelay` | delay of the second hand's shot when two weapons are held (dual wielding) | 0.05 to 0.75 | medium (name) |
| 18 | `b.20` | `bwp_fSpinUpTime` | spin-up time (the minigun barrel; also `WCF_SPINUP` weapons) | 0 to 0.6; the beam gun 0.25 (0.5 dm) | medium (name) |
| 19 | `b.21` | `bwp_fSpinDownTime` | spin-down time | 0 to 3; minigun 1.5, beam gun 0.75 (3 dm) | medium |
| 20 | `b.22` | `bwp_fSpinSpeed` | spin speed in degrees per second (the entity member is `en_fSpinningVelocityDeg`) | 500, minigun 550 (500 dm), 0 for grenade launcher and plasma rifle | medium |
| 21 | `b.23` | `bwp_pgppProjectileParams` | the **projectile parameter file**; null means a hitscan weapon | 16 of 35 weapon files | high (type) |

The columns are reproduced for every weapon by script section 3. Where `weapons.md` guessed "time between shots" for `b.30` this spec keeps the reading and **adds its basis**: the recoil duration is what the base class's `GetRecoilDuration` returns, and the recoil state (`WS_RECOIL`) is the interval after a shot.

#### 4.2 `SBaseScatterWeaponParams` (the 6 bullet weapons, 9 members)

| Pos | Data | Binary name | Meaning | Values (single player; deathmatch where different) | Confidence |
|---|---|---|---|---|---|
| 0 | `s.5` | `bswp_btBulletType` | the `BulletType` of the hitscan shot | 1 for Colt, Uzi, sniper; 2 for both shotguns and the minigun | medium |
| 1 | `s.1` | `bswp_fScatterDistance` | the distance at which the scatter radii apply | Colt 10, sniper 10; shotguns 100 (500); Uzi 100 (500); minigun 200 (750) | medium (name; the shooter's twin member is `en_fScatteringDistance`) |
| 2 | `s.2` | `bswp_fScatterRadiusMin` | smallest scatter radius at that distance | 0 (Uzi 0.1, dm 0) | medium |
| 3 | `s.6` | `bswp_fScatterRadiusMax` | largest scatter radius | Colt 0, sniper 0; auto shotgun 0.5 (3); double shotgun 1 (4.5); Uzi 2 (15); minigun 1 (22.5) | medium |
| 4 | named | `bswp_vPatternStretch` | stretches the pellet pattern in two axes | (1, 1) | high (named in the data) |
| 5 | `s.3` | `bswp_fBulletVelocity` | speed of a `BT_PHYSICAL` bullet (unused by the retail data's ray bullets) | 100 (shotguns), 130 | low |
| 6 | `s.4` | `bswp_fBulletRange` | range of the shot | Colt 100; auto and double shotgun 200 (500); Uzi 300 (500); minigun 300 (750); sniper 400 (1,500) | medium |
| 7 | named | `bswp_fBulletImpulse` | impulse on the target; -1 = the game default | -1 | high |
| 8 | named | `bswp_fBulletImpulseUpRatio` | the up component of the impulse | -1 | high |

This resolves `weapons.md`'s guess for `s.1` to `s.6` ("scatter spread and range grow in deathmatch"): **distance, minimum radius, maximum radius, velocity, range, bullet type**; the deathmatch rise of `s.1`, `s.4` and `s.6` is the rise of the scatter distance, the range and the maximum radius (**medium**, by the names).

#### 4.3 Other parameter classes

| Class | Members (binary names, data positions) | Notes |
|---|---|---|
| `SBaseWeaponRenderingParams` (8) | pos 0 `.1` `bwrp_vPosition` (Vector3f), 1 `.2` `bwrp_vAngle`, 2 `.3` `bwrp_fFOV` (0.53 to 1.23), 3 `.5` `bwrp_fBobbingMul` (0.4 to 1), 4 `bwrp_vPositionDoubleWeapon`, 5 `bwrp_vAngleDoubleWeapon`, 6 `bwrp_fFOVDoubleWeapon`, 7 `.4` `bwrp_fFrontClipDist` (0.1) | first-person placement alone and with two weapons held (`camera-view.md`: the placement was read there without names; the order rule names `.4` and `.5`); the 16 child classes add nothing |
| `SAutoShotgunWeaponParams` (2) | `aswp_ctBullets` (7), `aswp_slBulletDamage` (12 sp, 10 dm) | pellets and damage per pellet; sent to the owned shooter at the first shot |
| `SDoubleShotgunWeaponParams` (2) | `dswp_ctBullets` (14), `dswp_slBulletDamage` (12 sp, 7 dm) | as above |
| `SColtWeaponParams`, `SUziWeaponParams`, `SMiniGunWeaponParams`, `SSniperWeaponParams` | `cwp_slBulletDamage`, `uwp_slBulletDamage`, `mgwp_slBulletDamage` + five minigun sounds (`mgwp_pbsFire`, `Click`, `Spin`, `SpinUp`, `SpinDown`), `swp_slBulletDamage`, `swp_slBulletDamageInZoomMode`, `swp_fMinZoomFactor`, `swp_fMaxZoomFactor`, `swp_fFullZoomingTime` | named in the data already; values in `weapons.md` |
| `SBeamGunWeaponParams` (12) | pos 0 `.1` `bwp_slDamage` (14 sp, 7 dm), 1 `.2` `bwp_fDamageFrequency` (0.05), 2 `.3` `bwp_fDamageRadius` (0.1), 3 `.4` `bwp_fMaxRange` (100 sp, 50 dm), 4 `.16` `bwp_fDrainAmmoFrequency` (0.1), 5 `.6` `bwp_pmcModel` (`BeamLooping.mdl`), 6 `.8` `bwp_idHitMaterialEffect`, 7 `.15` `bwp_pefEffect` (`Beam_NoHit.pfx`), 8 `.9` `bwp_idFireAnim`, 9 `.10` `bwp_idSoundTerm`, 10 `.12` `bwp_idFirePressedSoundTerm`, 11 `.11` `bwp_idFireReleaseSoundTerm` | the beam is a continuous ray: damage every `DamageFrequency` seconds within `DamageRadius` up to `MaxRange`, ammunition drained every `DrainAmmoFrequency` seconds (the entity has the matching timers `en_tmNextDamage`, `en_tmNextAmmoDrain`); **medium** (names; the damage function was not read) |
| `SCircularSawWeaponParams` (9) | `cswp_fCuttingRange` 3, `cswp_fCuttingWide` 2, `cswp_fCuttingForceImpulse` 1, six sounds | named in the data |
| `SPlasmaRifleWeaponParams` (3) | `.1` `prwp_iAmpuleSegments` (3), `.2` `prwp_fRefillLength` (0.5), `.3` `prwp_fReleaseLength` (0.8) | the plasma rifle's ammunition is shown as **ampules**: the model has `iAmpuleSegments` fill steps per ampule, and the entity animates their fill level from the ammunition count (functions `UpdateAmpuleStrengths`, `ApplyAmpuleStrengths`); refill and release are animation lengths (seconds) (**medium**) |
| `SZapGunWeaponParams` (1) | `.1` `zgwp_ctMaxFirePowerWorthBullets` (6) | how many "bullets" a fully charged shot is worth (**medium**, name) |
| `SCannon`, `SGrenadeLauncher`, `SKlodovikGun`, `SRocketLauncher`, `SSeriousBomb`, `SCircularSawBlades`, `CFlamer` `WeaponParams` | none | everything they need is in the 22 base members and the projectile file |

### 5. Which `.ep` file has which class

(Script section 3.) **17 single-player files and 17 deathmatch files**, one class each: the 6 bullet weapons derive from `SBaseScatterWeaponParams`, the other 11 from `SBaseWeaponParams` (cross-checked with the 12 files of `SBaseScatterWeaponParams` in section 2). **16 rendering files**, one class each. The relation to `ep-schema.md`: its sections `SBaseWeaponParams`, `SBaseScatterWeaponParams`, `SBaseWeaponRenderingParams`, `SBeamGunWeaponParams`, `SPlasmaRifleWeaponParams`, `SZapGunWeaponParams` use the numbers; this page names them.

### 6. The weapon entities

#### 6.1 `CBaseWeaponEntity` (the base of every weapon)

**Role.** The first-person weapon object of a player: it plays the weapon's animations and sounds, runs the weapon state machine, fires on the fire button and pays the ammunition. **Net:** `NEB_DYNAMIC` (2), relevant distance unlimited (a 3.0e38 constant), 4 of its 34 members replicated: `en_fLastFirePower`, `en_wiWeaponIndex`, `en_iPlayer`, `en_bRightHandWeapon` (`replication.md`; confirmed by reading `GetNetBehavior` and `GetRelevantDistance` of this class and the table flags: **high** for the net behaviour and the relevance, medium for the member list).

**Members (`SBaseWeaponEntity`, 34; `#` = replicated, `@` = owned object).** Meaning from the name, **low** unless stated.

| Group | Members |
|---|---|
| links | `en_hmiWeapon@` (the model instance), `en_henOwner` (the owner entity), `en_pbwpWeaponParams`, `en_pbwrpWeaponRenderingParams` (the two `.ep` files), `en_hwaehAnimEventHandler@`, `en_hweppEffectPlacementProvider@`, `en_hsaPartMover@` (moves the spinning part), `en_iPlayer#` (which player), `en_wiWeaponIndex#`, `en_bRightHandWeapon#` |
| state | `en_wsState` (a `WeaponState`: it is what `CanBeChanged` and `IsRecoiling` test, medium), `en_tmLastFire`, `en_tmChargeStartTime`, `en_tmLastReload`, `en_ctShootsFiredFromRound` (shots since the last reload: `ShouldReload` reads it, medium), `en_fLastFirePower#` (fire power 0 to 1 of the last shot, medium), `en_bFirstInARawFiring` (set again by `OnFireReleased`, medium), `en_bAlternativeFirePressed` |
| firing options | `en_bAnimationDefinedRecoilDuration`, `en_bScalableRecoilDuration` (both read by `GetRecoilDuration`), `en_bDiscreteFirePower` (a partial power counts as 0: read in `DoTheFiring`, medium), `en_ulWeaponChargingType` (`WeaponChargingFlags`), `en_bSupportAutoAiming`, `en_idMuzzleFlash` |
| spinning | `en_fSpinningVelocityDeg`, `en_fSpinningPartAngleRad` |
| sound | `en_hss0@` to `en_hss3@` (four sound sources), `en_hssCharging@`, `en_iLastSoundSrc`, `en_csoSoundAnimEventsToTerminate`, `en_csoEffectAnimEventsToTerminate` (sounds and effects started by animation events, ended with them) |

**Exported methods (78 symbols, 71 named), grouped.** **(read)** = the function was read in full; **(name)** = from the name only.

| Group | Methods | What they do |
|---|---|---|
| life cycle | `OnCreate`, `OnDelete`, `OnStep`, `UpdateTemps`, `Reset`, `SetOwner`, `FetchOwner`, `ObtainWeaponParams` | create (find the parameter files, build the model instance and sound sources), delete, one simulation step (runs the state machine, **not read**), clear the temporary members, set the owner and read it back, return the weapon's parameter object; `Reset` and `ObtainWeaponParams` are named only (name) |
| net | `GetNetBehavior`, `GetRelevantDistance` | `NEB_DYNAMIC`; the unlimited distance (read) |
| state machine | `SetState`, `CanBeChanged`, `IsRecoiling`, `BringUp`, `PutDown`, `OnWeaponBroughtUp`, `OnWeaponPutDown`, `DoRecoil`, `OnRecoilEnd`, `DoCharging`, `OnChargingEnd`, `OnBoringEnd`, `ForceBoringAnimation`, `DoReload`, `OnWeaponReloaded`, `AnimationChanged` | enter a state (each state starts its animation or sound, section 9 rule 4), "can the weapon be switched away": **true unless the state is `WS_WAIT_FUSE_TO_BURN`**; "is it recoiling": the state is `WS_RECOIL`; bring the weapon up and put it down (play the activate / deactivate animation, enter `WS_BRINGING_UP` / `WS_PUTTING_DOWN`; the immediate form starts idle at once); the recoil, charging, boring-animation and reload steps and their end callbacks (**BringUp, PutDown, DoCharging, DoReload, SetState read**; the rest name) |
| firing | `OnFirePressed`, `OnFireReleased`, `OnAlternativeFirePressed`, `OnAlternativeFireReleased`, `FireReleasedInternal`, `CallFireStart`, `DoTheFiring`, `DoFiringsFromLastFireTime`, `OnFire`, `CanBeUsedForFiring`, `GetPreferredNextWeapon`, `Explode` | fire button down and up (alternative fire does nothing in the base class); **`DoTheFiring`**: needs an owner that accepts the attack and `CanBeUsedForFiring`, stores the fire power (0 when the power is discrete and below full), then calls `OnFire` (read); **`OnFire`**: raises a **threat sound** (the AI hears the shot within `ggp_fWeaponFireThreatDistance`, 100 in the data), tells the owner a weapon fired, and **for a player owner pays one shot of ammunition through the inventory and returns whether it could be paid; for any other owner returns 0** (read); `CanBeUsedForFiring`: asks the player owner whether this weapon index can fire (ammunition), false for other owners (read); `GetPreferredNextWeapon` returns "none" (read); `DoFiringsFromLastFireTime` catches up shots owed since the last step (name; a large function, not read); `Explode` empty in the base (read) |
| timing | `GetRecoilDuration`, `ShouldReload`, `IsZooming`, `ShouldShowCrosshair` | recoil duration (section 9 rule 2); reload due when `ctFiresToReload` is positive and the shots since the last reload have reached it; zooming false and crosshair true in the base (all read) |
| animation names | `GetRecoilAnim`, `GetIdleAnim`, `GetReloadAnim`, `GetActivateAnim`, `GetBoringAnim` | each returns the identifier of an animation of the weapon model (a fixed name; the zap gun picks the recoil animation by a random number) (read for the Colt and the zap gun) |
| tools and parts | `GetWeaponTool`, `MoveWeaponParts`, `CreatePartMover`, `GetShootingAttachment`, `GetShootingPlacement`, `AlterShootOrigin`, `AlterViewOrigin` | return the left or right hand's tool (read); move the spinning parts, create the part mover, name the model attachment bullets or projectiles leave from, compute the shoot placement (`GetShootingPlacement` is a very large function, not read), adjust the origin of the shot and of the view (name) |
| placement and render | `GetAbsPlacement`, `GetWeaponAbsPlacement`, `GetWeaponInViewMatrix`, `GetWeaponChargeDisplaceMatrix`, `AddRenderingCommand`, `Render`, `HideMuzzleFlash`, `OnAnimEventRenderMuzzleFlash` | where the weapon is in the world and in the view, the displacement while charging, draw it and the muzzle flash (name) |
| sound and effect events | `InitializeSoundSources`, `GetAvailableSoundSource`, `OnAnimEventProduceSound`, `OnAnimEventTerminateSound`, `OnAnimEventProduceEffect`, `OnAnimEventTerminateEffect`, `OnAnimEventShowPlayerWeaponAttachment`, `OnAnimEventHidePlayerWeaponAttachment`, `EvaluateWeaponActionEvents` | the animation events of the weapon models (`ans.md`): start and end a sound or effect, show or hide the attachment on the player's own model (name) |

**Virtual hooks.** `CBaseWeaponEntity` overrides 13 of the 102 slots of `CBaseEntity` (nine by their own exported names: the data-type function, create, delete, step, animation changed, absolute placement, net behaviour, relevant distance, update-temps; four that the table shows as shared stubs) and adds **58 new virtual functions** (state, animation names, the fire functions, recoil, charging, reload, render and the animation event hooks). The 58 are the hooks of the weapons below (**medium**: shared small functions show other classes' names in a few slots).

#### 6.2 The 17 weapon entities

Index = `WeaponIndex`. "Own members" are the binary's member table (`#` replicated: none; `@` owned). "Hooks" are the virtual functions the class overrides **by exported name**. "New" are virtual functions the class adds. Every class also overrides `mdGetDataType`.

| Weapon (index) | Class | Own members | Hooks overridden | New / extra methods |
|---|---|---|---|---|
| Circular saw (0) | `CCircularSawWeaponEntity` (23 symbols) | `en_fCuttingDamagePerSec`, `en_fCuttingApplyPeriod`, `en_hssEngine@`, `en_hssCutting@` | `OnCreate`, `OnDelete`, `InitializeSoundSources`, `CreatePartMover`, `AlterViewOrigin`, `PutDown`, `OnWeaponPutDown`, `BringUp`, `OnWeaponBroughtUp`, `OnFireReleased`, `OnFire`, `CallFireStart`, `GetRecoilDuration` | `CutWithSaw` (the cutting volume of `cswp_fCuttingRange` × `cswp_fCuttingWide`: damage per second and force; a large function, not read), `SetCuttingState`, `SetEngineState` (engine and cutting sounds) |
| Zap gun (1) | `CZapGunWeaponEntity` (11) | `en_ctFiredBullets` | `OnFire`, `GetRecoilAnim`, `ShouldReload`, `DoReload` | none. Launch velocity is `lerp(min, max, power)` (rule 6) |
| Auto shotgun (2) | `CAutoShotgunWeaponEntity` (11) | `en_henShooter@` | `OnCreate`, `OnFire` | owner functions `SetOwner`, `FetchOwner` shared with the other shooter weapons |
| Double shotgun (3) | `CDoubleShotgunWeaponEntity` (11) | `en_henShooter@` | `OnCreate`, `OnFire` | same |
| Uzi (4) | `CUziWeaponEntity` (13) | `en_henShooter@`, `en_bPlayingSam2Renovation` | `OnCreate`, `OnFire`, `ShouldReload`, `DoReload` | the only class besides the zap gun and the base with its own reload test (rule 8) |
| Minigun (5) | `CMiniGunWeaponEntity` (21) | `en_henShooter@`, `en_hssSpin@`, `en_hssFire@`, `en_hssClick@`, `en_hsaPartMover2@` | `OnCreate`, `OnDelete`, `InitializeSoundSources`, `CreatePartMover`, `MoveWeaponParts`, `OnFireReleased`, `OnFire`, `CallFireStart` | `CreatePartMover2`, `PlayClickSound`, `SetFireType` (`MGFireType`: starts or stops the fire or click sound, read), `SetSpinType` (`MGSpinType`: the spin-up, spin and spin-down sounds, read); `OnFire` sets spin type 2 (spinning) and fire type 1 (firing) |
| Rocket launcher (6) | `CRocketLauncherWeaponEntity` (8) | none | `OnFire` | none |
| Grenade launcher (7) | `CGrenadeLauncherWeaponEntity` (9) | none | `OnFire`, `DoCharging` | `DoCharging` also plays the charging animation at a rate that follows the charge (for a local owner only) |
| Plasma rifle (8) | `CPlasmaRifleWeaponEntity` (21) | none registered (the ampule records are internal) | `OnCreate`, `OnStep`, `PutDown`, `BringUp`, `OnFireReleased`, `OnFire`, `AnimationChanged`, `CallFireStart` | `GetAmmoQuantity` (the inventory's ammunition for this weapon), `GetMaxAmpuleValue`, `GetBetterAmpule`, `CanAllAmpulesBeFilled`, `UpdateAmpuleStrengths`, `ApplyAmpuleStrengths` (the ampule fill animation, rule 7) |
| Klodovik gun (9) | `CKlodovikGunWeaponEntity` (8) | none | `OnFire` | after the launch it tells the player puppet `OnKlodovicSentFlying` |
| Cannon (10) | `CCannonWeaponEntity` (8) | none | `OnFire` | launch velocity lerp (rule 6) |
| Serious Bomb (11) | `CSeriousBombWeaponEntity` (9) | none | `OnFire`, `Explode` | `OnFire` enters `WS_WAIT_FUSE_TO_BURN` after paying; `Explode` is read in rule 9 |
| Colt (12) | `CColtWeaponEntity` (12) | `en_henShooter@` | `OnCreate`, `OnFire`, `GetRecoilAnim` | owner functions as above |
| Sniper (13) | `CSniperWeaponEntity` (22) | `en_henShooter@`, `en_bZoomActive`, `en_bAvatarViewModeBeforeZoom`, `en_tmStartZooming`, `en_fFullZoomingTime`, `en_fMinFOVMultiplier`, `en_fMaxFOVMultiplier`, `en_fZoomRatio`, `en_hssZooming@`, `en_bLastFrameZooming`, five mask textures (`en_ptxSniperMask`, `Wheel`, `Eye`, `Arrow`, `Led`) | `OnCreate`, `OnDelete`, `OnStep`, `InitializeSoundSources`, `GetShootingPlacement`, `PutDown`, `Render`, `OnAlternativeFirePressed`, `OnFire`, `ShouldShowCrosshair`, `IsZooming` | `ActivateZoomMode`, `DeactivateZoomMode`: **alternative fire toggles the zoom**; the crosshair is hidden while zoomed (read) |
| Circular saw blades (none) | `CCircularSawBladesWeaponEntity` (7) | none | only `mdGetDataType` | **no behaviour of its own**: it is the entity class of the blades' parameter file (`SCircularSawBladesWeaponParams`); who creates it was not read |
| Beam gun (15) | `CBeamGunWeaponEntity` (26) | `en_tmNextDamage`, `en_tmNextAmmoDrain`, `en_bFirePending`, `en_bShouldFire`, `en_tmBeamSoundDelay`, `en_hmr3rdPersonBeam@` | `OnDelete`, `OnStep`, `CreatePartMover`, `PutDown`, `OnFire`, `OnFirePressed`, `OnFireReleased` | `Fire`, `FireStart`, `FireStop` (new virtual), `DoDamage` (the damage and ammunition drain tick, a large function, not read), `StartBeam`, `StopBeam`, `SetBeamLength`, `GetBeamPlacement`, `ShowBeamAttachment`, `HideBeamAttachment`, `GetModelInstance`, `Get3rdPersonModelInstance` (the beam is drawn in the player's own view and on the third-person model) |
| Flamer (16) | `CFlamerWeaponEntity` (11) | none | `OnCreate`, `OnDelete`, `GetShootingAttachment`, `OnFire` | flames leave from a fixed model attachment |

Weapon index 14 (`WI_COMBO_WEAPONS`) has **no weapon entity class**: dual wielding holds two ordinary weapon entities, one per hand (`en_bRightHandWeapon`).

### 7. The properties classes

`CBaseWeaponProperties` carries `ep_henOwner` and `ep_bRightHandWeapon` (the table of `SBaseWeaponProperties`); none of the 17 `C<Weapon>WeaponProperties` adds a member or a method. Nothing saves them (no file has them): they exist because the entity framework requires a properties class for every entity class (**medium**).

### 8. Weapon pickups and the other classes

| Class | What it is | Read |
|---|---|---|
| `CWeaponItemEntity` (`SWeaponItemEntity`; data: 4 members in 37 worlds, 3 in 9 older worlds, 46 files) | the pickup of a weapon. Members: `en_wiWeaponIndex` (which weapon), `en_pbwpWeaponParams`, `en_pbwpWeaponRenderParams` (its two `.ep` files), `en_slAmmoQuantity` (1 to 200 in the data). `OnCreate` looks up the two parameter files that belong to the weapon index and sets the two links when the lookup gives a path. `GetPickedUpAmmo` returns `en_slAmmoQuantity`. `IsNeeded(player)`: **true when the player does not have the weapon, otherwise true while the player's ammunition for it is below the maximum**. `OnPicked`: the base item pickup, then the player **receives the weapon and the ammunition** (the ammunition kind of the weapon index, the pickup quantity). `OnlyFirstPlayerPicks`: **in co-op, true when the session option `sgp_bWeaponStaysInCoop` is off** (the weapon is then consumed by the first player), otherwise the base item's rule. `GetPickupScore` is a constant-returning function (value not read). `EnablesDynamicShadow` is true | read (all seven), medium |
| `CCannonItemEntity` | the cannon's pickup (15 worlds): only `GetName` ("Cannon") differs from the base weapon pickup | read |
| `CCannonBallsItemEntity` | the cannon-balls ammunition pickup (19 worlds): parent `CAmmoItemEntity`; `GetName` ("Cannon balls"); `GetPickupAmount` is a constant-returning function (value not read) | read |
| `CComboWeaponsItemEntity` / `...Properties` (binary only) | the combo-weapons pickup. Properties and entity members: `..LeftHandWeapon`, `..AmmoQuantityForLeftWeapon`, `..RightHandWeapon`, `..AmmoQuantityForRightWeapon`, `..AutoOffTime`. `OnCreate`: copies them and shows on the item's model only the two child models named after the two weapons. `OnPicked`: **only when the global setting is not `CWU_NEVER`**: store the two weapons as the player's last combo weapons, make sure the player owns both (receiving a missing one), give the ammunition of both kinds, **start the combo mode for `AutoOffTime` seconds when the setting is `CWU_ITEM_CONTROLLED`**, then the base weapon pickup. The data's setting is `CWU_ALLOW_CUSTOM` (2), so in the retail data this item **starts no timed combo mode** (and no world contains one) | read, medium |
| `CAdjustWeaponMode` | a **developer tool**, not game play: it binds keys (F9 enables "weapon adjusting"; with Left Control the mouse moves the weapon left-right and up-down or in-out, rotates it, and with Left Shift changes the weapon's field of view) and a command "copy single to double weapon params"; it draws the adjusted view. It is how the rendering parameters (`SBaseWeaponRenderingParams`) were tuned. Methods `BindCommands`, `UnBindCommands`, `EvaluateControls`, `RenderView` | strings and names read, medium |
| `CMountedTool` | a record of a puppet's parameters: `mt_idTool` (an identifier such as `NPC_Zapgun_R`) and `mt_pctTool` (a `Ptr<CCharacterTool>`): **a tool a puppet carries mounted** (`SPuppetParams.pp_amtMountedTools`, 0 to 2 per file). The binary also declares an older member `mt_idToolObsolete` (by its name an obsolete identifier) that the data does not contain (low). `mdGetName` returns the tool's name | 9 files (script); vehicles have none (`vehicles.md`) |
| `CThrowDescription` | a record of a puppet's parameters: `td_idName` (`ThrowItem`, `ThrowFar`, `CommonThrow`, ...), three graphs (pitch, under-hit, prediction over distance) and `td_fMinThrowingVelocity`, `td_fMaxThrowingVelocity` (a 3e38 value means "no upper limit"). `CalculateParams(distance, ...)` evaluates the three graphs at a distance and returns **a pitch (the first graph's value times a constant), an under-hit value and a prediction value**. Used by the throwing behaviour (`CBAThrowPickedTool.batt_idThrowDescription`, `misc-schema.md`) of the enemies, **not by the player's weapons** | 15 files (script); medium |

### 9. Rules read from small functions

Every row is **medium** (one reading of an exported function) unless said.

| # | Rule | From |
|---|---|---|
| 1 | **Net:** weapon entities are `NEB_DYNAMIC` (value 2) and always relevant (distance 3.0e38); only the four members of 6.1 are replicated; no weapon subclass adds a replicated member (table flags of the 9 subclasses that have a table) | `GetNetBehavior`, `GetRelevantDistance`, tables; `replication.md` agrees (two readings → **high** for the net behaviour) |
| 2 | **Recoil duration** = `min + (max - min) × fire power` of the last shot (`bwp_fMinRecoilDuration`, `bwp_fMaxRecoilDuration`, `en_fLastFirePower`); **but** when the weapon is not a `WCF_CHARGE` weapon or its recoil is not scalable, **and** the minimum is 0 (or below a tiny constant), the duration is **the length of the recoil animation** | `GetRecoilDuration` |
| 3 | **Reload:** `ShouldReload` is true when `ctFiresToReload > 0` and the shots fired since the last reload ≥ `ctFiresToReload`. `DoReload` (when `ctFiresToReload > 0` and at least one shot was fired, and a minimum interval since the last reload has passed): enter `WS_RELOAD`, reset the shot counter, play the reload animation (rescaled when `bwp_fReloadDuration > 0`). The zap gun keeps its own fired-bullet count (how it grows with the fire power was not read) and tests it against `ctFiresToReload` the same way; its files have 0. In the retail data **only the Colt reloads (every 6 shots)** | `ShouldReload`, `DoReload`; data (script) → **high** that only the Colt has a magazine |
| 4 | **States:** leaving `WS_BORING` informs the owner; entering `WS_IDLE` plays the idle animation; entering `WS_CHARGING` starts the charging sound (and leaving it stops it); entering `WS_CANT_FIRE` tells the owner with this weapon's handle; `WS_BORING` plays a boring animation (when there is none, back to idle at once); `WS_WAIT_FUSE_TO_BURN` plays an animation and schedules a callback after its length (the bomb's fuse). `BringUp(0)` = `WS_BRINGING_UP` with the activate animation, `BringUp(nonzero)` = skip the animation and be idle; `PutDown` = `WS_PUTTING_DOWN` with the deactivate animation (and, unless forced, only when `CanBeChanged` allows) | `SetState`, `BringUp`, `PutDown` |
| 5 | **Fire:** every fire function first calls the base `OnFire`; **a subclass does nothing when it returns 0**, so a player with no ammunition, or any non-player owner, never shoots. Order: threat sound → owner notified → ammunition paid (`DrainAmmoFromWeapon`) → the subclass launches | `OnFire` of the base and of 12 subclasses |
| 6 | **Hitscan weapons** (Colt, Uzi, minigun, auto and double shotgun, sniper): the weapon sets the shooter's **damage-inflictor weapon** to its own index (Colt 12, Uzi 4, minigun 5, auto shotgun 2, double shotgun 3, sniper 13) and calls the shooter's fire function with a **bullet description** record that the weapon builds (from its placement and parameters). Both shotguns send their pellet count and damage to the shooter once, at the first shot. **Projectile weapons** call the **owner puppet's** launch function with `bwp_pgppProjectileParams`, the damage-inflictor weapon (rocket launcher 6, plasma rifle 8, Klodovik gun 9, flamer 19) and a description record; the cannon, the grenade launcher and the zap gun use the variant that also takes a **launch velocity `min + (max - min) × fire power`** | `OnFire` of 12 subclasses; the launch-velocity rule agrees with the data (nonzero in exactly these three files) → **high** for the velocity rule, medium for the rest |
| 7 | **Plasma rifle ampules:** the weapon's ammunition count *q* (from the inventory) is shown as *N* ampules of *S* segments (*S* = `prwp_iAmpuleSegments`, *N* an entity member that is not a parameter; the data does not give it): *q* is spread over the ampules in turn, each showing `q / N` segments (integer division) plus one more for the first `q mod N` ampules in rotation, as a fill between 0 and 1 of `segments / S`; each shot lowers the fill of the next ampule in rotation by `1 / S` (not below 0) and the entity animates the fill levels toward their targets | `OnFire`, `GetMaxAmpuleValue`, `GetBetterAmpule`, `UpdateAmpuleStrengths`, `ApplyAmpuleStrengths` (**low**: partly read) |
| 8 | **Uzi reload:** the Uzi's `ShouldReload` is true only when the entity member `en_bPlayingSam2Renovation` is set, a global switch (identity not read) is on and the game is single player, and then only **after more than 49 shots**. `DoReload` runs the base reload under the same conditions, and the base reload needs `ctFiresToReload > 0`, which is 0 in both Uzi files: **with the retail data the Uzi never reloads**. The member name suggests a re-release mode that the data does not enable (low) | `CUziWeaponEntity::ShouldReload`, `DoReload`, `CBaseWeaponEntity::DoReload` |
| 9 | **Serious Bomb:** `OnFire` (after paying) enters `WS_WAIT_FUSE_TO_BURN`; `Explode` (after the fuse) applies a world effect to **every puppet of the world info's list that is in state 1** (taken to mean alive; the state's name was not read), tells the world-info entity `OnSeriousBombExploded` (the remote call of `replication.md`), tells the player `OnUsedSeriousBomb`, then returns to idle. The effect's damage and what it excludes were **not read** | `OnFire`, `Explode` |
| 10 | **Weapon pickups:** rules of section 8; an enemy-kill weapon drop or the player's dropped weapons use the same `CWeaponItemEntity` (`players-session-classes.md`, `CreateWeaponItemEntity`) | `CWeaponItemEntity` functions |

## Invariants

1. Every `S<name>WeaponParams` / `SBaseWeaponParams` file has the **same 22 members in the same order**; a reader assigns the binary's names of section 4.1 by position (checked on 35 files).
2. A weapon file's **bullet-weapon test** is "its class derives from `SBaseScatterWeaponParams`" (12 files = 6 weapons × 2 files, checked by script). A weapon file whose projectile link (`b.23`) is null is hitscan or special, a non-null one launches that projectile file (16 of the 35 files that carry `SBaseWeaponParams`: 17 single player, 17 deathmatch and the leftover beam-gun copy).
3. Every weapon has a deathmatch file and (but the blades) a rendering file; **17 + 17 + 16** (script).
4. `ctFiresToReload` is 6 for the Colt and 0 elsewhere (17 of 17 single-player and 17 of 17 deathmatch files, script).
5. The three weapons with a nonzero launch velocity are the three that read it (grenade launcher, cannon, zap gun).
6. The enumerations' value lists are as in section 3 (no gap: states 1 to 10, bullet types 0 to 3, hands 0 and 1).
7. Reject a weapon entity class outside the 17 + base; reject a `WeaponIndex` outside -1 to 16 (14 is a valid index with no entity class).

## Open questions

Ranked by what blocks an implementer. Task IDs are **new** (appended to `docs/roadmap/T01-reverse-engineering.md`).

1. **The weapon state machine and its timing** (`OnStep`, `DoRecoil`, charging, bring-up, boring, reload gap, minigun spin, sniper zoom, the shots owed after a long frame): only the small functions are read. **T01-117.**
2. **The hitscan shot:** what the shooter does with the bullet description (scatter model with the section 4.2 members, bullet types and trails, damage type, the bullet description record, the owned shooter's creation and ownership), shotgun pellet setup. Joins T01-107 (shooters and bullets). **T01-118.**
3. **Projectile launch and charging:** the owner puppet's launch function (the two variants), what "fire power" is (0 to 1 over the charging time, `bwp_fChargingAutoFireTime`), `ctMaxFirePowerWorthBullets` of the zap gun, the flamer and Klodovik launches. **T01-119.**
4. **Special weapons:** the beam gun's damage tick (`DoDamage`), the saw's cutting volume (`CutWithSaw`) and its damage numbers, the Serious Bomb's effect, the plasma rifle's ampule arithmetic, who creates the blades entity. **T01-120.**
5. **The view feel members** (`bwp_fOffsetX` to `bwp_fOffsetZExponent`), the double-weapon placement and delay, sniper zoom presentation and the rendering parameters' use (with `camera-view.md`). **T01-121.**
6. **Pickups and combo:** the table that maps a `WeaponIndex` to its two parameter files, `GetPickupAmount` of ammunition items, the co-op stay rules together with the player side, the combo-weapons mode (`CWU_*`), the remaining `C<Weapon>ItemEntity` classes (T01-108 owns their inventory). **T01-122.**
7. **Throw descriptions and mounted tools** (enemies' throws; graph units; the obsolete identifier): **T01-123.**
8. **Run-time checks on the original** of the numbers of this page (recoil periods, the Colt's 6-shot magazine, launch velocities, ampule fill, the Uzi mode): **T01-124.**
9. Smaller: the cheat set 49,151 of `player-rules.md` leaves out the combo-weapons bit (14) and the flamer bit (16) of `WeaponFlags`: whether "give all weapons" really omits the flamer is to be settled with T01-114; and the fire power of a shot that is not charged (the value `DoTheFiring` receives from the attack) was not read, which rule 2 and test vector 3 depend on (T01-117).

## Test vectors

All from the retail install via `tools/re/weapon_classes_verify.py`; values are single player (deathmatch in brackets where different).

1. **Position rule, Colt.** `SColtWeaponParams` has one member, `cwp_slBulletDamage`, class base `SBaseScatterWeaponParams`; in `SBaseWeaponParams` the number `5` is at position 3 and the binary's name there is `bwp_ctFiresToReload`; the Colt file has `b.5` = 6 and every other weapon file has 0.
2. **Launch velocity.** Position 15 and 16 are `bwp_fLaunchVelocityMin` and `bwp_fLaunchVelocityMax`: grenade launcher 10 and 130, cannon 40 and 140, zap gun 40 and 25, every other weapon 0 and 0. A shot of fire power 0.5 of the cannon launches at 40 + (140 − 40) × 0.5 = **90**.
3. **Recoil duration.** The sniper's file has `b.30` = 0 (1 in deathmatch) and `b.31` = 0 (1): in single player, if the sniper's recoil is not scalable (not read), the recoil lasts as long as the recoil animation (rule 2). The Colt's minimum is 0.5 and its maximum 0: for an uncharged shot (fire power 0, **assumed**) the recoil lasts 0.5 s. The plasma rifle's minimum and maximum are both 0.15 (0.1), so its recoil is 0.15 s whatever the fire power.
4. **Scatter members.** Minigun: bullet type 2 (moving trail), scatter distance 200 (750), radius 0 to 1 (0 to 22.5), range 300 (750), velocity 130; Colt: type 1, distance 10, radii 0 and 0, range 100.
5. **Class counts.** 83 exported classes, 780 exported functions, 44 in the data (40 as `S` twin, 4 under the `C` name), 39 only in the binary; the data classes of the family: `SBaseWeaponParams` 22 members in 35 files, `SBaseScatterWeaponParams` 9 in 12, `SBaseWeaponRenderingParams` 8 in 16, `SWeaponItemEntity` 4 in 46, `CMountedTool` 2 in 9, `CThrowDescription` 6 in 15.
6. **Enumeration.** `WeaponState` 5 = `WS_CHARGING`; `CanBeChanged` is false only in state 10; `IsRecoiling` is true only in state 8.
7. **Pickup rule.** A weapon pickup with `en_wiWeaponIndex` 12 (Colt) for a player who holds the Colt and has the maximum Colt ammunition is **not needed** (`IsNeeded` false); with the weapon missing it is needed whatever the ammunition.
