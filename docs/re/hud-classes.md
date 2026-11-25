# HUD classes of `Sam2Game.dll` (T01-111, part of T01-026)

- **Roadmap task:** T01-111 (part 11 of the class inventory T01-026; the other parts are T01-102 to T01-113). Companion pages: `hud-classes-simple.md` (the base classes and the 25 "simple" elements), `hud-classes-special.md` (the other elements), `hud-classes-params.md` (the parameter classes and the database).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/hud_classes_verify.py` reproduces from the data: which of the 95 exported classes are saved (47) and under which name, their base classes and member counts, that every saved class is in exactly one file, the database of 44 entries and its numbers, and the **member names** (31 data classes, 141 binary-declared members, all agree on count, position and type); **high** for the class hierarchy, the virtual-slot layout and the creation order of the 44 elements (two readings each: registrations and virtual tables, constructors and the creation function); **medium** for every rule that rests on one reading of a small function (visibility, stacking, the command dispatch, what each simple element shows) and for the member meanings that rest on a name; **low / open** for pixel-exact drawing and for the per-frame behaviour of the large `Render` functions (parked, T01-186 to T01-195). **Nothing was run.**

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `rsc-ctl.md` and `rsc-ctl-schema.md` (the `.rsc` container and the generated member reference), `players-session-classes.md` (the player puppet that owns the HUD; sections 5.3 and 9 cover two more HUD classes), `replication.md` (net behaviour), `weapon-classes.md` (the member-order rule, the weapon enumerations), `localization.md`.

## Summary

`Sam2Game.dll` exports **95 HUD classes** (797 exported functions, 95 `md_pdtDataType` data symbols) that make up the player's head-up display, plus **four more** that `players-session-classes.md` counted with the player (`CPlayerDeathHudElement`, `CVotingInfoHudElement` and their parameter classes). What a reader needs to know first:

1. **A HUD element is a small object that draws one thing, driven by a parameter object from the data.** There are **48 element classes** (`C...HudElement`; 46 here, 2 in the player part; 44 of them are created) and **47 parameter classes** (`C...HudElementParams`; 45 here, 2 in the player part; 44 are used). The parameter objects are saved in one database file (`Databases/HudParams.rsc`, `hud-classes-params.md`); the elements are **never saved**.
2. **A HUD container owns the elements.** The class `CHUD` (data name `SHUD`) is **not exported**; it holds the list of the 44 elements, draws them in creation order on a **640 x 480 virtual canvas** and passes the HUD commands (weapon keys, Netricsa scrolling) to every element. Each player puppet owns one HUD and caches handles to seven elements (section 5).
3. **Two layers of elements.** `CSimpleHudElement` is the base of the 25 elements that show a number, an icon and up to two bars (health, armour, air, ... the power-ups, the ammunition); the other 19 elements draw something special (weapons list, the Netricsa panel, the scanner, damage flashes, message queues). All of them override the same **11 virtual slots** of `CHudElement` (section 4).
4. **Everything is local.** None of the 345 members whose flags were read carries the replication flag, the HUD is not created from a world file and the player's cached handles are not replicated. The only network traffic is **one client-to-server call**, `CPlayerPuppetEntity::OnHudCommand`, that sends a weapon-slot command to the server (section 9).
5. **Ammunition and weapon icons** are loaded by two global tables, `CAmmoIcons` and `CWeaponIcons`, one texture per value of `AmmoIndex` (12) and `WeaponIndex` (17); the file name is the enumeration caption with blanks replaced by `_` (section 10).
6. **The 47 saved classes use 45 exported names plus two renamed ones**: the data calls `CSeriousStrengthHudElementParams` `CStrengthSimpleHudElementParams` and `CSeriousJumpHudElementParams` `CJumpSimpleHudElementParams`; the survey of T01-026 counted 45 saved classes because those two do not match by name.

## Evidence

- **Export table.** The 95 classes are those of the T01-026 split (`P9 hud`); their exported functions total **797**, equal to the survey: 93 classes with the standard seven symbols (default and copy constructor, destructor, assignment, stream read, virtual-function table, `mdGetDataType`; `CAmmoIcons` and `CWeaponIcons` have six, no table), and **134 named methods** besides the standard set, **133 of them on element classes** and one (`GetHudElementParameters`) on the database. Every class is in `Sam2Game.dll` and in the mod DLL `InSamnity2Game.dll` (the mod build of the same classes, T13); no other module of the install contains any of the names (string search of every `.dll`, `.exe` and `.module` of `Bin`; **high**).
- **Registrations and member tables.** The DLL registers every script class with a record (name, parent name, editor caption, the name under which the data stores it) and a static member table (name, type reference, flags). 95 of the 99 names were read from the registrations (the four elements `CAutoAimHudElement`, `CFreezingHudElement`, `CObjectActionHudElement`, `CSavingHudElement` are registered with a name string that is not delimited like the others; their parent follows from the virtual table and the object size), **50 member tables** were read (17 element classes, 32 data-side classes with `CNetricsaHudElementParams` and `CHUD`; 46 of them with their flags). Offsets and sizes were read but **are not recorded** (`conventions.md`).
- **Virtual-function tables.** The table of each of the 48 element classes was compared slot by slot with `CHudElement` (11 slots) and `CSimpleHudElement` (20 slots) (section 4). Identical empty functions are shared by the linker, so a slot can show another class's name; such slots are not used for meaning.
- **Small functions read in full** (**medium**, one reading each): `ShouldBeVisible`, `UpdateTemps`, `Initialize`, the `PrepareRenderingParameters` of the 22 elements named below, `OnHudCommand` and `TryToChangeWeapon` of the weapon elements, `Say`, `OnAddLife`, `OnFragsCountChanged`, `AddMessage`, `GetHudElementParameters`, the HUD container's creation, update and render loops (internal, found from the callers of the element constructors), the initialisers of the two icon tables, and `CPlayerPuppetEntity::InitializeHeadUpDisplay` and `::OnHudCommand`. The large `Render` functions (about 40 to 800 lines each) were **not** read as wholes.
- **Data.** `tools/re/hud_classes_verify.py` reads the type tables of all 8,063 format-8 files (`--all`, about 4 minutes; without it the 112 `.rsc` files, a few seconds), the two HUD files and the icon folders and prints every count of sections 2 and 3 and of `hud-classes-params.md`.
- **Binary build.** The `Sam2Game.dll` used for the other specs; no other build was compared. **Not checked:** nothing was run, no screenshot taken; the formulas of the bars and texts, the drawing code of the large elements, the behaviour of the container's update function and the unexported helper classes (`CTextQueue`, `CHUD`'s methods) were not read.

## Layout / rules

### 1. The classes and the survey's numbers

| Group | Classes | Exported | Saved as (data) | Named methods |
|---|---|---|---|---|
| base element | `CHudElement` (data name `SHudElement`) | 1 | not saved | 7 |
| simple base | `CSimpleHudElement` (`SSimpleHudElement`) | 1 | not saved | 14 |
| power-up base and its 7 children | `CPowerUpHudElement` and `CSeriousStrength/Speed/Jump/Damage/ScoreHudElement`, `CInvulnerabilityHudElement`, `CInvisibilityHudElement` | 8 | not saved | 1 + 0 |
| 14 simple elements | Adrenaline, Air, Armor, Bombs, Boss, ComboWeapons, Counter, Frozen, Health, Lives, Points, Time, Vehicle, VehicleAmmo (`C...SimpleHudElement`) | 14 | not saved | 20 |
| ammunition | `CAmmoHudElement`, `CLeftAmmoHudElement`, `CRightAmmoHudElement` | 3 | not saved | 5 |
| online notification | `COnlineNotificationHudElement` | 1 | not saved | 3 |
| 18 other elements | AutoAim, BackTemplate, Conversation, Damage, EnemyPurged, ExtraLife, FragsCounter, Freezing, HorizontalWeaponsList, ItemPicked, Netricsa, NetricsaIcon, ObjectAction, Saving, Scanner, TimeRemain, WeaponsList, WeaponsMenu | 18 | not saved | 83 |
| **elements** | | **46** | | **133** |
| parameter classes | `CHudElementParams`, `CSimpleHudElementParams`, `CAmmoHudElementParams` and one per element (42) | **45** | **45 saved** (47 with the two resources) | 0 |
| database | `CHudElementsParamsDatabase` | 1 | saved (1 file) | 1 (`GetHudElementParameters`) |
| bar parameters | `CSimpleHudBarParams` | 1 | saved (1 file) | 0 |
| icon tables | `CAmmoIcons`, `CWeaponIcons` | 2 | not saved, no table | 0 |
| **total** | | **95** | **47 saved, 48 not** | **134** |

Counts: 46 + 45 + 1 + 1 + 2 = 95; 93 x 7 + 2 x 6 + 134 = 797. **Survey reconciliation** (**high**, script): the survey's "45 in the data" is the 45 parameter classes; the saved total is **47** with the database and the bar parameters; "46 `*HudElement`" and "45 `*HudElementParams`" are exact. The four classes of the player part (`CPlayerDeathHudElement`, `CVotingInfoHudElement` and their parameters) are in the same database, which has 44 entries = 42 of this family + those 2.

Not exported but registered by the DLL and relevant here (**medium**, table names only): `CHUD` (data name `SHUD`, not saved; 6 members: `hud_henOwner`, `hud_csoHudElements`, `hud_vFirstRelElmPos`, `hud_fVerticalOffset`, `hud_bWeaponsMenuActive`, `hud_bNetricsaActive`), the three configuration records `CHudTextConfig` (saved as `SHudNumberConfig`), `CHudIconConfig` (`SHudIconConfig`), `CHudBarConfig` (`SHudBarConfig`) and the shared block `CCommonSimpleHudElmParams`. Four of these names (`hud_bNetricsaActive`, `hud_bWeaponsMenuActive`, `hud_fVerticalOffset`, `hud_vFirstRelElmPos`) also appear in the `hud_` list of `console-variables.md`; the table shows that they are **members of `CHUD`**; whether they are also registered as console variables was not checked.

### 2. Names in the data

Elements are never saved. A parameter class is saved under its **`S` twin** (24 classes: `CHealthSimpleHudElementParams` is saved as `SHealthSimpleHudElementParams`, and likewise the base classes, ammunition, weapon lists, scanner, damage, back template), under **its own name** (19 classes) or **renamed** (2, above). The DLL's registrations give each data name; the script prints the table (section 1 of its output). The element name decides which parameters an element uses (section 6): the lookup appends `Params` to the element's own type name, which for the 24 `S`-twin elements is the data name (`SHealthSimpleHudElement` + `Params`) and for the others the exported name; the two renamed power-ups are found through their exported name.

### 3. Hierarchy (**high**: registrations; virtual tables for the four unregistered parents)

```
CScriptObject
  CHudElement (SHudElement)                       the 11-slot base
    CSimpleHudElement                             number + icon + two bars
      CAdrenalineSimpleHudElement, CAirSimpleHudElement, CArmorSimpleHudElement, CBombsSimpleHudElement,
      CBossSimpleHudElement, CComboWeaponsSimpleHudElement, CCounterSimpleHudElement, CFrozenSimpleHudElement,
      CHealthSimpleHudElement, CLivesSimpleHudElement, CPointsSimpleHudElement, CTimeSimpleHudElement,
      CVehicleSimpleHudElement, CVehicleAmmoSimpleHudElement, COnlineNotificationHudElement
      CAmmoHudElement  -> CLeftAmmoHudElement, CRightAmmoHudElement
      CPowerUpHudElement -> CSeriousStrengthHudElement, ...SpeedHudElement, ...JumpHudElement, ...DamageHudElement,
                            ...ScoreHudElement, CInvulnerabilityHudElement, CInvisibilityHudElement
    CAutoAimHudElement, CBackTemplateHudElement, CConversationHudElement, CDamageHudElement,
    CEnemyPurgedHudElement, CExtraLifeHudElement, CFragsCounterHudElement, CFreezingHudElement,
    CHorizontalWeaponsListHudElement, CItemPickedHudElement, CNetricsaHudElement, CNetricsaIconHudElement,
    CObjectActionHudElement, CSavingHudElement, CScannerHudElement, CTimeRemainHudElement,
    CWeaponsListHudElement, CWeaponsMenuHudElement                         (+ CPlayerDeathHudElement, CVotingInfoHudElement)
  CHudElementParams (SHudElementParams)           position, render mode, render-on-camera
    CSimpleHudElementParams                       stacking flag, text/icon/bar configurations, bar parameters
      the same 15 + 7 simple children, CAmmoHudElementParams -> Left, Right
    the 18 (+2) other parameter classes
CScriptObject  CHUD (SHUD)  the container (not exported)
CResource  CHudElementsParamsDatabase, CSimpleHudBarParams
```

The four elements without a registered name (`CAutoAimHudElement`, `CFreezingHudElement`, `CObjectActionHudElement`, `CSavingHudElement`) have exactly the virtual table of `CHudElement` with their own overrides and the same object size, so their parent is `CHudElement` (**medium**). The parents of the power-up parameter classes are `CSimpleHudElementParams` (there is no `CPowerUpHudElementParams`; **high**).

### 4. The virtual slots (11 for `CHudElement`, 20 for `CSimpleHudElement`; **high** for the order, **medium** for the roles)

Slots are numbered from 0. The roles come from the exported method names that sit in the slots of the classes that override them (two or more classes each), the default being the base's.

| Slot | Method | Base default | Role |
|---|---|---|---|
| 0 | deleting destructor | | (standard) |
| 1 | `mdGetDataType` | | the class's type (standard) |
| 2, 3 | inherited from `CScriptObject` | | names not recovered |
| 4 | `UpdateTemps` | looks the element's parameters up and stores them | re-fetch the parameters of the element from the database |
| 5 | `OnHudCommand(HudCommand)` | empty | react to a HUD command (section 9) |
| 6 | `Initialize(Handle<CHUD>)` | stores the HUD handle, calls `UpdateTemps`, copies `render on camera` | one-time set-up when the HUD creates the element |
| 7 | `Reset` | empty | clear transient state (message queues, strings) |
| 8 | `PrepareRenderingParameters` | empty | compute what to show this frame and the element's own visibility wish |
| 9 | `ShouldBeVisible(int)` | the visibility rule of section 7 | decide whether to draw |
| 10 | `Render(Vector2f *, float *)` | returns 0 | draw; returns 1 when something was drawn |
| 11 (simple) | `GetIconTextureOverride` | returns none | an element-specific icon instead of the configured one |
| 12 | `GetIconStretch(icon cfg)` | the configured stretch | a time-dependent icon scale |
| 13 | `GetTextStretch` | 1.0 | a time-dependent text scale |
| 14 | `RenderBar` | draws one or two bars | |
| 15 | `RenderBarPart` | draws one part of a bar | |
| 16 | `RenderIcon` | draws the icon | |
| 17 | `RenderText` | draws the text | |
| 18, 19 | `AdjustColorForPhase1`, `AdjustColorForPhase2` | empty | modify the bar colour (the boss element flashes in phase 1) |
| 20 (ammunition) | `GetPlayerHand` | right hand (1) | which hand's weapon the ammunition element shows (`CLeftAmmoHudElement` returns 0, `CRightAmmoHudElement` 1) |

The weapon elements add slots after 10: `CWeaponsListHudElement` 11 `TryToChangeWeapon`; `CHorizontalWeaponsListHudElement` 11 `TryToChangeWeapon`, 12 `PrepareWeaponsArrays`, 13 `RenderBar`, 14 `RenderWeaponIcon`, 15 `RenderGlowedEdge`, 16 `RenderWeaponQuantity`, 17 `GetWeaponColor`; `CWeaponsMenuHudElement` 11 `TryToChangeWeaponInDirection`, 12 `SetNewWeapon`, 13 `PrepareWeaponsArrays`, 14 `RenderWeapon`; `CExtraLifeHudElement` 11 `OnAddLife`; `CTimeRemainHudElement` has 13 slots (two further slots that no exported method names). The other methods (`Say`, `OnItemPicked`, `OnEnemyPurged`, `OnFragsCountChanged`, `Clear`, `AddMessage`, `Show`, `Hide`, `GetCountOfWeaponSlots`, the Netricsa `Render*` helpers) are **non-virtual**.

### 5. The container, creation and the player's handles (**medium**: one reading of each function)

- **Creation.** The HUD creates **44 elements in this order** (the order of the constructor calls; the elements are drawn in this order): BackTemplate, AutoAim, Freezing, Lives, NetricsaIcon, Armor, Health, Air, Adrenaline, Frozen, Boss, Time, SeriousStrength, SeriousJump, SeriousDamage, SeriousScore, SeriousSpeed, Invulnerability, Invisibility, Counter, ComboWeapons, Bombs, Vehicle, VehicleAmmo, Points, Scanner, RightAmmo, LeftAmmo, ExtraLife, FragsCounter, TimeRemain, ItemPicked, Damage, ObjectAction, PlayerDeath, Saving, EnemyPurged, HorizontalWeaponsList, WeaponsList, OnlineNotification, WeaponsMenu, Conversation, Netricsa, VotingInfo. The abstract classes `CHudElement`, `CSimpleHudElement`, `CAmmoHudElement` and `CPowerUpHudElement` are never created. **High** that these are the 44 classes (the same 44 as the database's, script) and that the order is that of the code; **medium** that it is the draw order.
- **Update.** Each frame the container goes through the elements **one by one** and for each calls `PrepareRenderingParameters` and then, if the element is drawn, `Render`; an element is drawn when `he_bIsVisible` is set and either the draw function's pass argument is 0 or the element's `he_bRenderOnCamera` is set (the argument's exact meaning is open); the `Render` arguments are the container's `hud_vFirstRelElmPos` (the position of the last stacked element) and `hud_fVerticalOffset` (the accumulated height) (**medium**).
- **Canvas.** The container sets an orthographic projection of **0..640 x 0..480**, switches off the depth buffer, depth write and alpha test and selects a blend mode before drawing, and keeps a scale factor derived from the aspect ratio that the elements multiply their sizes by (`hic_fStretch` x scale). **Medium**; the 44 positions of the database are all inside 640 x 480 (script, **high**).
- **Lookup.** The container finds an element by its class name; `CPlayerPuppetEntity::InitializeHeadUpDisplay` (called from `OnCreate` and `PostWorldChange`) stores handles to **seven elements** in the player puppet: Conversation (`en_hheConversation`), NetricsaIcon (`en_hheNetricsaIcon`), FragsCounter (`en_hheFragsCounter`), ItemPicked (`en_hheItemPicked`), ExtraLife (`en_hheExtraLife`), PlayerDeath (`en_hhePlayerDeath`) and EnemyPurged (`en_hheEnemyPurged`). The puppet calls `Say`, `Show`/`Hide`, `OnFragsCountChanged`, `OnItemPicked`, `OnAddLife`, `AddMessage`, `OnEnemyPurged` on them (the exported callers are `SayPlayerInternal`, `SayPuppetInternal`, `SayNetricsaInternal`, `ShowNetricsaIconInternal`, `HideNetricsaIconInternal`, `OnItemPicked`, `ShowPlayerDeathMessage`, `OnEnemyPurged`, `OnGameBooted`). The weapon, Netricsa and simple elements are not cached; they read the player through the HUD's owner.
- **Reset.** The HUD calls `Reset` on all elements (a loop over the element list); `OnGameBooted` clears the frags counter.

### 6. Parameter lookup (**high** for the pairing, **medium** for the mechanism)

`CHudElement::UpdateTemps` asks the global database object for the parameters of the element and keeps the result only if it is a `CHudElementParams`. The database's `GetHudElementParameters(element)` builds the name of the element's class plus the suffix `Params`, finds that data type by name and returns the **first object of that class in its list of 44**; when the type or the object is missing it prints `Can't find data type for params class: %1 to create parameters.` and returns nothing. **High** for the pairing: the 44 element classes created in section 5, each with `Params` appended, name exactly the 44 parameter classes of the database (script); the suffix itself is not visible in the reading. `Initialize` of the elements that cache textures copies them from the parameters and, when the parameters are of the wrong class, prints `Invalid hud element params.`; an element without parameters draws nothing.

### 7. Visibility (**medium**: `ShouldBeVisible` read in full; **high** for the enumeration)

`RenderHudElement` (member `hep_rheRender`; 44 of 44 entries have 1):

| Value | Name | Caption | Visible when |
|---|---|---|---|
| 0 | `RHE_HIDDEN` | Never | never |
| 1 | `RHE_AUTO` | Auto | the element wants to be shown **and** the global HUD switch is on |
| 2 | `RHE_IF_HUD_IS_RENDERED` | If hud is rendered | the global HUD switch is on |
| 3 | `RHE_ALWAYS_WHEN_NEEDED` | Always when need it | the element wants to be shown |
| 4 | `RHE_ALWAYS` | Always | always |

"Wants to be shown" is the argument that the element's `PrepareRenderingParameters` passes to `ShouldBeVisible`; **without parameters** the element is visible when it wants to be and the switch is on. The **global HUD switch** is a single flag that is read by this function and written only by the cheats menu's "toggle HUD" button (`CCheatsMenuScreen::btnToggleHud_OnClick`; its initial value was not read). Because every entry has `RHE_AUTO`, **an element is visible exactly when it wants to be and the cheat-menu switch is on**.

### 8. Stacking of simple elements (**medium**: `CSimpleHudElement::Render` read in full)

`CSimpleHudElement::Render(position, offset)` takes the parameters' position and the running vertical offset. If the entry's `shep_bRelToPrevious` is set **and the last drawn element's position equals this entry's position**, the element is drawn lower by the accumulated offset. It then draws the bar, the icon and the text (three calls in the order bar, icon, text, through slots 14, 16 and 17; each returns the height that part used) and, **if the flag is set**, adds the largest of the three heights to the offset; **if the flag is not set** the offset is reset to 0. It returns 1 when it drew, 0 when it has no parameters. The result is the four stacks of `hud-classes-params.md` section 4: the seven power-ups under (4, 100); Frozen, Counter, ComboWeapons under (195, 45); Air, Adrenaline, Time under (195, 55); Boss alone at (210, 45). Because the offset and the last position are kept in the container between elements and nowhere reset in the draw loop that was read, an element drawn after a stack with the same position would be shifted; whether the container resets them elsewhere is open (T01-186).

### 9. HUD commands and net behaviour

`HudCommand` (23 values, **high**: the table was read as value, name pairs): 1 to 10 `HC_SLOT_1` to `HC_SLOT_10`, 11 `HC_WEAPON_RIGHT`, 12 `HC_WEAPON_DOWN_RIGHT`, 13 `HC_WEAPON_DOWN`, 14 `HC_WEAPON_DOWN_LEFT`, 15 `HC_WEAPON_LEFT`, 16 `HC_WEAPON_UP_LEFT`, 17 `HC_WEAPON_UP`, 18 `HC_WEAPON_UP_RIGHT`, 19 `HC_DESC_SCROLL_UP`, 20 `HC_DESC_SCROLL_DOWN`, 21 `HC_OBJ_SCROLL_UP`, 22 `HC_OBJ_SCROLL_DOWN`, 23 `HC_NETRICSA_USE`.

- `CPlayerPuppetEntity::OnHudCommand(command)` **on a client** sends the command to the server as a reliable remote call (`replication.md` lists it among the client-to-server calls); **on the server or in a local game** it passes the command to every element of the player's HUD (the container's broadcast, slot 5).
- Elements that react (**medium**): the two weapon lists (commands 1 to N where N = `GetCountOfWeaponSlots`: **10** if the global game parameters' `ggp_cwuComboWeaponsUsing` is 1, else **9**; the data value is 2, so 9) call `TryToChangeWeapon(slot)`; the weapon menu (commands 11 to 18 = direction 0 to 7) calls `TryToChangeWeaponInDirection`; the Netricsa element handles 19 and 20 (scroll the description up and down) and 21 and 22 (scroll the objective list up and down) by `nhep_pixScrollSpeed` x `nhe_fMinScrollDelay` per command, accepted at most once per `nhe_fMinScrollDelay` seconds per command, and 23 (`HC_NETRICSA_USE`: toggles `nhe_bShowLevelDesc`).
- **Replication.** None of the **345 members of the 46 tables whose flags were read** (17 element classes and 29 data-side classes) carries `SMF_NETUPDATE` (0x4000; `replication.md`); the flag values seen are 0 (260 members), 8, 10, 11, 0x1000 (2 members, among them `he_hhepParams`, the parameters handle that `UpdateTemps` rebuilds), 0x2005 (3) and 0x40008 (62 resource pointers). The flags of the four tables read later (`CHUD`, the database, the bar parameters, the shared block) were not read. The player puppet's member list (`players-session-classes.md`) marks `en_hhudHeadUpDisplay` and the seven `en_hhe...` handles as owned, not replicated. The server has the HUD of every player puppet (the creation runs in `OnCreate`), so a weapon-slot command changes the weapon on the server through the element's own logic (**medium**).
- **Per-player.** Each element reads the player through `he_hhud` (the container) and its owner (`hud_henOwner`); there is **no global HUD state** except the visibility switch of section 7 and the two icon tables.

### 10. Icon tables (`CAmmoIcons`, `CWeaponIcons`; **medium** for the reading, **high** for the file match)

Both are plain structures (no registration, no members that the data names) held in two global pointers (`sam_paiAmmoIcons`, `sam_pwiWeaponIcons`) and created by `samInitializeAmmoIcons` / `samInitializeWeaponIcons`, destroyed by the `samCleanup...` twins. Each holds a growing array of texture pointers. The initialiser builds, for each enumeration value, the name `Content/SeriousSam2/Textures/Interface/AmmoIcons/<caption>.tex` (or `.../WeaponIcons/<caption>.tex`) where `<caption>` is the **enumeration caption with blanks replaced by `_`**, and loads the texture. `samGetAmmoIcon(AmmoIndex)` and `samGetWeaponIcon(WeaponIndex)` return it; the ammunition element draws `samGetAmmoIcon(samGetAmmoForWeapon(weapon in the hand))`, the two list elements and the weapon menu draw `samGetWeaponIcon`. In one branch (a build whose name equals `Sam2Renovation`) the weapon initialiser also forms a `<name>_screen.tex` name (no such file is in the install).

Script check (**high**): all **12** `AmmoIndex` captions (`Blades`, `Bullets`, `Cannon balls`, `Grenades`, `Hand grenades`, `Klodovik`, `Plasma`, `Rockets`, `Serious bomb`, `Shells`, `Sniper bullets`, `Napalm`) and all **17** `WeaponIndex` captions (`Circular_Saw` ... `Flamer`) have a file; the folders hold **14** and **21** files, the extra ones (`Copter_Rockets`, `Crossbow_Arrows`; `Circular_saw_blades`, `No_weapons`, `plasma_beam_gun`, `voodoo_gnaar`) are not named by any enumeration value. Whether the loop covers the `none` value (and so `No_weapons`) was not read.

### 11. Strings the HUD classes translate (names of the binary's translation keys; the text after `=` is the English default)

`ETRSHudElement.EnemyPurged.GotBoredAlone` (got bored and killed itself), `...GotBoredWithOthers` (got bored and killed themselves), `ETRSExtraLifeHudElement.Message` (EXTRA LIFE!), `ETRSHudElement.FragsCounter.FragsLeft` (frags left), `ETRSHudElement.PickedItem.MaxLimit` (full), `ETRSSavingHudElement.Saving` (Saving...), `ETRSHudElement.TimeRemain.MinutesLeft` (min left), `...SecondsLeft` (sec left), `...TimeExpired` (Time expired), the Netricsa keys `ETRSHUD.Netricsa.*` (Description, Statistic, Exit, Level, EstTime, Playtime, Secrets, Lives, Kills, Difficulty, Score, Lite, MissionObjectives) and the voting keys `ETRSVotingInfo.*` (NextMap, RestartMap, KickPlayer, TimeBanPlayer, VoteInProgress, VoteFailed, VotePassed). The tables are in `localization.md`.

## Invariants

1. The database has **44** entries; the container creates **44** elements; the two sets of class names correspond one to one by the rule element name + `Params` (44 of 44, script for the names).
2. **47** of the 95 exported classes are saved; **each saved class is in exactly 1 of 8,063 files**; no element class is saved (46 of 46 not in the data).
3. **31 of 31** data classes with a binary table agree with it on member count, position of named members and types (one prefix exception).
4. None of the 345 members whose flags were read carries the replication flag (0 of 345).
5. The slot order of `CHudElement` (11) and `CSimpleHudElement` (20 = 11 + 9) holds for all 46 element classes (they have 11, 12, 13, 15, 18, 20 or 21 slots, each equal to its parent's count plus the new methods named above).
6. A reader must reject an element whose parameters are of another class (the original prints `Invalid hud element params.`) and must treat a missing parameter entry as "not drawn".

## Open questions

1. **The container**: where and when its draw function is called, whether `hud_vFirstRelElmPos` and `hud_fVerticalOffset` are reset per frame, what the draw pass flag means exactly, the initial value of the global HUD switch: **T01-186**.
2. **Simple elements' formulas**: the bar fractions (health, armour, air, time, counter), the texts built from them (the float-to-integer steps were not recoverable from the reading), the bar and text drawing, the shake and the pulses: **T01-187**.
3. **Netricsa panel**: the 13 `Render*` helpers and the 96 parameters, the statistics, the objective list, the sign and the scroll rules: **T01-188**.
4. **Scanner** (the radar): range, classes of dots, the five placements, the level counter that hides it: **T01-189**.
5. **Weapon elements**: which of the three weapon elements is used when, the slot table (below), the selection animation, the combo-weapons modes: **T01-190**.
6. **Special elements' drawing**: damage flashes, freezing, auto-aim, object action, saving, player death, voting, online notification: **T01-191**.
7. **Message-queue elements** (conversation, item picked, enemy purged, extra life, frags counter, time remain): the text-queue class, accumulation windows, text effects: **T01-192**.
8. **Run-time check on the running original** (positions, stacks, which values appear): **T01-193**.
9. **Icon tables**: the loop bounds, fallback for a missing file, the `none` icon, the `Sam2Renovation` branch: **T01-194**.
10. **Engine pieces the HUD uses** but this DLL does not own (`CGradient`, `CFontInstance`, `CTextEffectConfiguration`, the text queue): **T01-195**.

## Test vectors

1. **Visibility.** An entry with `hep_rheRender` = 1 and an element that wants to be shown: visible when the global switch is on, hidden when it is off. An entry with 4 is visible with the switch off. An entry with 0 is never visible. (Section 7.)
2. **Stack.** Two entries at (4, 100) with the stacking flag set, the first drawing a part of height 40: the first is drawn at y = 100; the second is drawn at y = 140 if its position equals the position of the last drawn element. An entry with the flag cleared is drawn at its position and resets the offset. (Section 8.)
3. **Commands.** With `ggp_cwuComboWeaponsUsing` = 2 the weapon list accepts commands 1 to 9; command 10 is ignored. With the value 1 it accepts 1 to 10. Command 13 (`HC_WEAPON_DOWN`) goes to the weapon menu as direction 2. (Section 9.)
4. **Names.** `CSeriousStrengthHudElementParams` is the data class `CStrengthSimpleHudElementParams` (editor caption `Serious strength`, base `CSimpleHudElementParams`, no members); it is the 11th entry of the database at (4, 100) with the stacking flag set, an icon `SeriousStrength.tex` with stretch 0.225 at offset (-3, 5), a text at (35, 12) and a bar of length 70.
5. **Default weapon slots** (both weapon lists, `WeaponIndex` values; first, second): 1: saw (0), none; 2: zap gun (1), Colt (12); 3: double shotgun (3), auto shotgun (2); 4: minigun (5), Uzi (4); 5: rocket launcher (6), grenade launcher (7); 6: plasma rifle (8), beam gun (15); 7: sniper (13), Klodovik gun (9); 8: cannon (10), flamer (16); 9: Serious bomb (11), none; 10: combo weapons (14), none. A slot shows the first weapon if the player has it, else the second; if only the second is owned it moves into the first position (read from the initialiser and `PrepareWeaponsArrays`, **medium**).
