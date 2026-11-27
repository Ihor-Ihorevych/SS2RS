# Player, session and inventory classes of `Sam2Game.dll` (T01-102, part of T01-026)

- **Roadmap task:** T01-102 (part 1 of the class inventory T01-026; the other parts are T01-103 to T01-113)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** only for the counts and relations that `tools/re/players_session_classes_verify.py` reproduces from the data (which of the 52 classes are saved, under which name, how many files carry the resource classes, how the player parameter members line up with the binary's names); **medium** for everything read from the binary (the exported class and method names, the member tables and their flags, the inheritance, the enumeration names and values, the tables of ammunition and the rules read from small functions) and, as of T01-221 (`CPlayerBrainEntity::{ProcessAutoAiming,ProcessViewRayCasting,IsAutoAimingSupported}`, read in full), for the auto-aim candidate-selection algorithm, the release mechanism and the weapon-flag check, because each rests on one reading and **nothing was run**; **low** for what a method does when it is described only from its name (marked "(name)"), and for the items listed in "Open questions".

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `ep.md` and `ep-schema.md` (the `.ep` files), `replication.md` (net behaviour and the replicated members of 33 class tables), `save-game.md` (the chapter save and the stored status classes), `player-rules.md` and `inventory.md` (the numbers in the data), `weapons.md`.

## Summary

`Sam2Game.dll` exports **52 classes** that make up the player, the multiplayer session and the player's belongings (740 exported symbols; 351 of them are the standard set every class has, 389 are named methods). Only **10 of the 52 are saved in any data file**, and they are all parameter or resource classes. The other **42, among them every entity class of this family (the player puppet, the player brain, the game info entity, the voting info entity), the inventory and the save/statistics records, appear in no type table of the 8,063 files**: they exist only in the binary, which is why this spec has to be built from the binary's own exported names and member tables.

What a reader needs to know first:

1. **Names.** The code exports `C<Name>` for every class, the member tables and the data use `S<Name>` for many entity and parameter classes (`CPlayerPuppetParams` is exported, `SPlayerPuppetParams` is what a `.ep` file stores). The two names are the same class (section 1).
2. **The three layers of an entity.** For each entity concept there are up to three classes: the **entity** (`...Entity`, the live object), its **properties** (`...Properties`, the editable values a level stores for it) and its **parameters** (`...Params`, a shared `.ep` file). The player puppet has all three; the game info and voting info entities have entity and properties; none of the four entity classes of this family appears in any `.wld` (they are created at run time by the game; how and when is not read here, T01-091).
3. **The player's belongings** are not separate entities: the inventory (`CPlayerInventory`) is a member **embedded in the player puppet**, and the 26 `*InventoryItem` classes are **obsolete** (nothing constructs them but their own registration code).
4. The binary declares the **names and values of the enumerations** the data stores as bare numbers (17 weapon indices, 12 ammunition kinds, 7 power-up indices, the game types, the difficulty levels, the 12 chapter boot types), the **ammunition tables** (cap per kind, cost per shot, kind used per weapon) and the **default weapons** (section 7). Several rules that `player-rules.md` and `save-game.md` left open come from these.

## Evidence

- **Exported names.** The 11,610 exports of `Sam2Game.dll` were listed; the 932 exported classes were grouped by name (the family boundaries are in T01-026) and the 52 classes of this family were taken with their exported symbols (constructors, destructor, assignment, stream operator, vftable, `mdGetDataType` and the named methods). Method names are quoted as exported.
- **Member tables.** The DLL contains static tables of class members (name, a data-type reference, a byte offset, a flags word). 227 of them were read; those of `SPlayerPuppetEntity` (94 members), `SPlayerBrainEntity` (31), `SGameInfoEntity` (8) and `SPlayerPuppetParams` (24) were read in full, those of the save and inventory records (`CPlayerInventory`, `CPlayerSave`, `CStoredChapterStatus`, `CStoredLevelStatus`, `CGameStats`) by member-name prefix. The flag meanings (`SMF_NETUPDATE` 0x4000, `SMF_OWNED` 0x2000, `SMF_TEMPORARY` 0x1000) are those of `replication.md`. Byte offsets and sizes were read but are **not recorded** (`conventions.md`; T01-063).
- **Inheritance.** The C++ parent of a class was read from the exported constructor (which parent constructor it calls first) and checked against the **virtual function table**: the vtable of a class was compared slot by slot with its parent's (for example 377 of the 445 slots of `CLeggedPuppetEntity` are unchanged in `CPlayerPuppetEntity`). Two readings; **medium**, because identical small functions are shared by the linker, so a slot can show another class's name (such slots are not used here).
- **Rules** (section 7) come from reading **small functions** (getters and table lookups); each is `medium` (one reading), and where two functions of the family or the data agree this is said.
- **T01-221** additionally read, in full, `Sam2Game.dll`'s `CPlayerBrainEntity::ProcessAutoAiming`, `ProcessViewRayCasting` and `IsAutoAimingSupported`; `en_bAutoAiming`'s own byte offset was cross-checked against `CPlayerPuppetEntity::OnAutoAiming`'s toggle, and the weapon-flag offset against `weapon-classes.md`'s already-named `en_bSupportAutoAiming`.
- **Enumerations.** The names and values of `WeaponIndex`, `AmmoIndex`, `PowerUpIndex`, `EGameType` (the `GT_` names), the difficulty enumeration (`GD_`) and the chapter boot type (`CBT_`) are in data tables of the DLL (value, name pairs). They were read twice, with the disassembler's string and data view and with an independent script that parses the file's sections; both give the same lists. **Medium** (the same bytes), but consistent with the data in three places (section 7.1).
- **Data.** `tools/re/players_session_classes_verify.py` reads the type tables of all 8,063 format-8 files (about 2 minutes) and prints the counts of sections 2 and 9.
- **Binary build.** `Sam2Game.dll` of the install used for the other specs (the same one as `replication.md` and `save-game.md`); no other build was compared.
- **Not checked.** Nothing was run. No method was read in full except the small functions of section 7. The sentence for a method that is described only from its name is marked "(name)". Behaviour of the weapons, projectiles, puppets and brains these classes use is the subject of T01-103 to T01-107.

## Layout / rules

### 1. Names and layers

| Rule | Statement | Evidence | Confidence |
|---|---|---|---|
| **C name exported, S name stored** | Of the 932 exported classes of the DLL, 178 have the same name in the data and 249 appear in the data with the first letter changed from `C` to `S` (`CPuppetEntity` is exported, `SPuppetEntity` is saved); 505 are not in the data at all. In this family: 7 classes keep their `C` name in the data (section 9), 3 appear as the `S` twin (`SCameras`, `SKnockOutParams`, `SPlayerPuppetParams`), 42 are not in the data | script (section 9), export table | high for the counts, medium that the twins are the same class |
| **The twin is the same class** | The member table of `SPlayerPuppetEntity` names as its parent the *C* name `CLeggedPuppetEntity`; the exported constructor of `CPlayerPuppetEntity` calls the constructor of `CLeggedPuppetEntity` directly and no constructor named `SPlayerPuppetEntity` is exported; there is no second class `SPlayerPuppetEntity` in the export table. So `SPlayerPuppetEntity` is the **data name** of the C++ class `CPlayerPuppetEntity`, not a base class | tables, constructor, vtable | medium |
| **Entity, properties, params** | `Entity` = live object, `Properties` = the values stored with a level entity, `Params` = a shared `.ep` resource. Here: player puppet (E, P, `Params`), player brain (E, P), game info (E, P), voting info (E, P). The `Properties` classes of the family have **no member table** and only the standard 7 exported symbols: they are empty shells, the entities carry their own members | tables, exports | medium |
| **Standard symbols** | Every class exports the default and the copy constructor, the assignment operator, the stream operator that reads it from a data stream (`operator>>` with a `CSafeDataType`), the vftable (when it has virtual functions) and `mdGetDataType` (returns its type descriptor); most also export the destructor. 351 of the 740 exported symbols of the family are these | export table | medium |

### 2. The 52 classes

"Saved as" is the name of the type in the data (script). "Parent" is the C++ parent read from the constructor and the vtable. "Net" is `GetNetBehavior` from `replication.md` (`PRED` predicted, `DYN` dynamic, `STAT` static, `-` none). "Symbols" is the number of exported symbols (named methods in brackets).

| Class | Kind | Parent (C++) | Saved as (data) | Symbols (methods) | Net | Section |
|---|---|---|---|---|---|---|
| `CPlayerPuppetEntity` | entity | `CLeggedPuppetEntity` | not saved | 201 (194) | PRED | 3 |
| `CPlayerPuppetProperties` | properties | `CLeggedPuppetProperties` | not saved | 7 (0) | - | 1 |
| `CPlayerPuppetParams` | `.ep` params | `CLeggedPuppetParams` | `SPlayerPuppetParams` | 7 (0) | - | 9 |
| `CPlayerBrainEntity` | entity | `CBrainEntity` | not saved | 76 (69) | PRED | 4 |
| `CPlayerBrainProperties` | properties | `CBrainProperties` | not saved | 7 (0) | - | 1 |
| `CGameInfoEntity` | entity | `CBaseEntity` | not saved | 80 (72) | DYN | 5 |
| `CGameInfoProperties` | properties | `CBaseProperties` | not saved | 7 (0) | - | 1 |
| `CVotingInfoEntity` | entity | `CBaseEntity` | not saved | 26 (19) | DYN | 5 |
| `CVotingInfoProperties` | properties | `CBaseProperties` | not saved | 7 (0) | - | 1 |
| `CInventoryBase` | record | none | not saved | 6 (0) | - | 6 |
| `CPlayerInventory` | record, embedded in the puppet | none | not saved | 30 (24) | (member of the puppet) | 6 |
| `CPlayerSave` | record | none | not saved | 8 (1) | - | 8 |
| `CStoredChapterStatus` | record | none | not saved | 9 (2) | - | 8 |
| `CStoredLevelStatus` | record | none | not saved | 6 (0) | - | 8 |
| `CGameStats` | record, embedded in the game info entity | none | not saved | 8 (2) | (member of the game info) | 8 |
| `CBaseInventoryItemObsolete`, `CWeaponInventoryItem`, `CAmmoInventoryItem` and the 23 classes of section 6.3 | obsolete records | `CBaseInventoryItemObsolete` | not saved | 7 each (0) | - | 6.3 |
| `CPlayerDeathHudElement`, `CVotingInfoHudElement` | HUD elements | HUD base (T01-111) | not saved | 11 (4), 9 (2) | - | 5.3 |
| `CPlayerDeathHudElementParams`, `CVotingInfoHudElementParams` | HUD parameters | `SHudElementParams` | same names | 7 (0) each | - | 9 |
| `CCameras` | sub-object of `.ep` | none | `SCameras` | 6 (0) | - | 9 |
| `CKnockOutParams` | sub-object of `.ep` | none | `SKnockOutParams` | 6 (0) | - | 9 |
| `CPlayerPuppetCrosshairs` | resource | `CResource` | same name | 7 (0) | - | 9 |
| `CGlobalPlayerParams` | resource part | `CResource` | same name | 7 (0) | - | 9 |
| `CGameDifficulties`, `CDifficultyParams`, `CPowerUpDurations` | parameter records | none | same names | 4, 5, 4 (0) | - | 9 |

Parents of the properties classes: the constructors of `CPlayerBrainProperties`, `CGameInfoProperties` and `CVotingInfoProperties` call the `CBaseProperties` constructor and that of `CPlayerPuppetProperties` the `CPuppetProperties` constructor, while the table's naming and the vtables point to `CBrainProperties` and `CLeggedPuppetProperties`; the code of these constructors can be shared, so the column shows the table's name (**medium-low**).

Counts: 52 classes = 4 entities + 4 properties + 1 params + 6 records and 26 obsolete items (`CBaseInventoryItemObsolete` + 25 `*InventoryItem`) + 2 HUD elements + 9 parameter and resource classes (the HUD parameter classes are counted there). **Confidence:** the names, symbol counts and the saved/not-saved split are high/medium as in the header; the parent column is **medium**.

### 3. `CPlayerPuppetEntity` (the player's body)

**Role.** The legged puppet that a player controls: it holds the player's weapons in two hands, the inventory, the power-up manager, the head-up display, the text queue and the Netricsa conversation objects, and it renders the player's own view. In multiplayer every machine has one per player. **Net:** `NEB_PREDICTED` (3), relevance distance 3.0e38 (effectively unlimited), update priority 10.0 (the three values read from three one-line functions; **medium**). Its members: section 3.3. Its parameters: `SPlayerPuppetParams` (4 `.ep` files, section 9).

#### 3.1 Exported methods (201 symbols, 194 named), grouped

Each row says what the group is for. **(name)** = from the names only; the others were also read (section 7) or are described in an earlier spec.

| Group | Methods | What they do |
|---|---|---|
| life cycle | `OnCreate`, `OnNetCreate`, `OnBoot`, `OnDelete`, `OnShutDown`, `OnStep`, `PreWorldChange`, `PostWorldChange`, `CollectForSortedIteration`, `ResetDynamicMembers`, `UpdateTemps`, `DeleteBrain`, `DeleteWeapons` | the standard entity hooks (created, created from the network, booted for a chapter, deleted, one simulation step, before and after a world change, include in sorted iteration, reset the members that are not saved, delete the brain and the weapon entities) (name) |
| net | `GetNetBehavior`, `GetUpdatePriority`, `GetRelevantDistance`, `PreSendUpdate`, `PostReceiveUpdate` | net behaviour class, priority, distance, and the hooks before sending and after receiving a replicated update (`replication.md`) |
| weapons in hand | `GetWeaponInActiveHand`, `GetWeaponIndexInPlayerHand`, `GetCurrentWeaponIndex`, `GetActiveWeaponIndex`, `SetCurrentWeapon`, `SetNewWeapon`, `SetNewWeaponToPlayerHand`, `SelectWeapon`, `ToggleToWeapon`, `OnNextWeapon`, `OnPrevWeapon`, `OnToggleLastWeapon`, `FindFavoriteWeaponThatCanBeUsed`, `FindNextWeaponThatCanBeUsed`, `FindPreviousWeaponThatCanBeUsed`, `GetStrongestWeaponThatCanBeUsed`, `SetStrongestWeaponThatCanBeUsed`, `SetFavoriteWeaponThatCanBeUsed`, `CanBeUsedForFiring`, `CanChangeWeapon`, `CanFireFromWeaponNow`, `PutDownWeapons`, `OnWeaponPutDown`, `OnWeaponBroughtUp`, `OnWeaponRecoiled`, `SetWeaponFOVMultiplier` | select and change the weapon, step through the usable weapons (next, previous, last, strongest, favourite), test whether the weapon has the ammunition to fire (the rule is in section 7.3), and the events when a weapon is put down, brought up or recoils (name) |
| dual wielding and combo | `ToggleDualWielding`, `ToggleDualWielding_NoWeaponsList`, `ToggleDualWielding_WeaponsList`, `IsDualWielding`, `ToggleLeftWeaponChoosing`, `ActivateChoosingOfLeftHandWeapon`, `DeactivateChoosingOfLeftHandWeapon`, `IsChoosingOfLeftWeaponActive`, `StartComboWeaponsMode`, `StopComboWeaponsMode`, `IsComboWeaponsModeActive`, `GetComboWeaponsModeDuration` | hold one weapon per hand, choose the left-hand weapon (a time-limited mode, `ggp_fTimeToChooseLeftWeapon`), and the combo mode in which the two hands fire different weapons (name; the 3 s window is in `player-rules.md`) |
| firing and use | `DoAttack`, `StopAttack`, `DoReload`, `OnFireButtonDown`, `IsWeaponFiringPressed`, `GetWeaponFiringButton`, `IsFlippingFireButtons`, `CantBeUsed`, `OnHudCommand` | start and stop firing, reload, which button fires which hand (the "flip fire buttons" preference swaps them) (name) |
| inventory and items | `ReceiveWeapon`, `ReceiveAmmo`, `ReceivePowerUp`, `ResetPowerUps`, `IsPowerUpActive`, `OnPowerupActive`, `OnItemPicked`, `DropWeapon`, `DropCurrentWeapons`, `DropPowerUp`, `DropActivePowerUps`, `CreateWeaponEntity`, `CreateWeaponItemEntity`, `CreatePowerUpItemEntity`, `OnObjectTaken`, `OnObjectDropped`, `StoreToolsBeforeRide`, `RestoreToolsAfterRide` | pick up and drop weapons, ammunition and power-ups, create the weapon entity when a weapon is received, drop weapon and power-up items when the player dies, carry objects, keep the tools while riding a vehicle (name; `ReceiveWeapon`/`ReceiveAmmo`/`ReceivePowerUp` hand over to the inventory, section 6) |
| damage and death | `OnReceiveDamage`, `OnDamageTaken`, `GetReceiveDamageMultiplier`, `GetApplyDamageMultiplier`, `IsInvulnerable`, `IsInvulnerableAsRider`, `CanReceiveFallingDamage`, `CanReceiveDamageFromMaterial`, `ApplyEnvironmentDamage`, `RecivesKOForceForceImpulse`, `DoDying`, `GetKillScore`, `GetCorpseStadyPeriod`, `GetHealingInfo`, `GetInitialMaxHealth`, `GetNominalHealth`, `OnIKilledPuppet`, `OnKlodovicSentFlying`, `ShowPlayerDeathMessage`, `RenderDeathMessage` | take and apply damage, the damage multipliers, invulnerability, health numbers, kill score and the death message (several read, section 7.2) |
| movement | `GetSpeedMultiplier`, `GetMovementSpeedMultiplierForMaterial`, `GetJumpMultiplier`, `GetAirControlPeriod`, `GetAirControlFactor`, `GetAirControlVelocityMultiplier`, `ResetAirControl`, `AutoJump`, `ToggleSprint`, `IsSprinting`, `GetTimeSinceStartedSprinting`, `GetTimeSinceStoppedSprinting`, `UpdateSwimWobbling`, `DoFlocking`, `CollideWithCarriedObject`, `GetCollisionCategory`, `IsOnProteins` | speed, jump and air control, sprint, auto-jump, and what the power-ups change (section 7.2); the collision category is the string id of the entity class (`GetCollisionCategory`) |
| view | `GetViewOrigin`, `GetLookDirEul`, `GetFOVMultiplier`, `AnimateViewOrigin`, `UpdateViewOriginInterpolator`, `UpdateBobbingValues`, `UpdateDampingValues`, `UpdateFreezingExhalation`, `RenderView`, `InjectRenderingCommands`, `ShouldShowCrosshair`, `RenderOverlay`, `RenderOverlayFadings`, `RenderScores`, `RenderPlayersList`, `RenderTargetedPlayerName`, `RenderCheatsInfo`, `GetBeaconsAndObjectivesFadeRatio`, `OnCameraStarted`, `InitializeHeadUpDisplay` | the first-person view: eye position and bobbing, damping of the view, freezing breath, the overlay, the scoreboard, the player list and the cheat information drawn on top (name) |
| Netricsa, speech and text | `SayNetricsa`, `SayNetricsaInternal`, `SayNetricsaNet`, `SayPlayer`, `SayPlayerInternal`, `SayPlayerNet`, `SayPuppet`, `SayPuppetInternal`, `SayPuppetNet`, `ShowNetricsaIcon`, `ShowNetricsaIconInternal`, `ShowNetricsaIconNet`, `HideNetricsaIcon`, `HideNetricsaIconInternal`, `HideNetricsaIconNet`, `ClearConversation`, `ClearConversationInternal`, `ClearConversationNet`, `SetText`, `SetTextFade`, `ClearText`, `ClearTextFade`, `ClearAllTexts`, `ClearAllTextsFade`, `IsNetricsaScreenActive`, `IsWeaponsMenuActive` | the three-level pattern `X`, `XInternal`, `XNet`: the script-callable function, the local action and the remote call that tells the clients (the `...Net` ones are in the server-to-clients list of `replication.md`) (name) |
| game events | `OnAddLife`, `OnAddLives`, `OnSetLives`, `OnExtraLifeAwarded`, `OnEnemyPurged`, `OnSecretFulfilled`, `OnChapterFinished`, `OnLevelFinished`, `OnGameBooted`, `OnThrewGrenade`, `OnUsedSeriousBomb`, `ReceiveFrags`, `GetCustomData`, `SetCustomData`, `GetAchievmentProgressInfo` | lives, score events, achievements, frags (name) |
| cheats | `OnGod`, `OnGhost`, `OnFly`, `OnGiveAll`, `OnHealth`, `OnArmor`, `OnInvisible`, `OnAutoAiming` | the client-side effect of each cheat after the server accepted the request (`replication.md`: requested by a client, executed by the server, broadcast) |
| profile | `SetProfileData`, `FetchProfileData` | copy the player's preferences (auto aim, view bobbing, flip fire buttons, crosshair, third person, sprint toggle, auto jump) between the profile and the puppet, from where they are replicated (name) |
| model | `GetPuppetModel`, `UpdateCurrentCharacter` | which model the player is drawn with (the multiplayer character) (name) |

#### 3.2 Virtual functions overridden

A vtable comparison with `CLeggedPuppetEntity` (445 slots; `CPlayerPuppetEntity` has more) shows: **68 slots are overridden and 68 are new** (**medium**). The overridden slots whose function is exported under this class are: `mdGetDataType`, `OnCreate`, `OnNetCreate`, `OnDelete`, `OnStep`, `PreWorldChange`, `PostWorldChange`, `CollectForSortedIteration`, `GetNetBehavior`, `GetUpdatePriority`, `GetRelevantDistance`, `PreSendUpdate`, `PostReceiveUpdate`, `GetViewOrigin`, `OnShutDown`, `OnBoot`, `OnReceiveDamage`, `UpdateTemps`, `ResetDynamicMembers`, `GetPuppetModel`, `GetCollisionCategory`, `DeleteBrain`, `StoreToolsBeforeRide`, `RestoreToolsAfterRide`, `OnObjectDropped`, `IsOnProteins`, `OnWeaponPutDown`, `OnWeaponRecoiled`, `DoAttack`, `StopAttack`, `CantBeUsed`, `GetInitialMaxHealth`, `ReceiveWeapon`, `ReceiveAmmo`, `ReceivePowerUp`, `GetHealingInfo`, `IsInvulnerable`, `IsInvulnerableAsRider`, `OnDamageTaken`, `GetReceiveDamageMultiplier`, `GetApplyDamageMultiplier`, `CanReceiveFallingDamage`, `GetKillScore`, `DoDying`, `GetCorpseStadyPeriod`, `GetSpeedMultiplier`, `IsFlippingFireButtons`, `GetLookDirEul`, `GetMovementSpeedMultiplierForMaterial`, `GetFOVMultiplier`, `InjectRenderingCommands`, `ShouldShowCrosshair`, `RenderOverlay`, `GetBeaconsAndObjectivesFadeRatio`, `RenderView`, `ResetAirControl`, `GetAirControlPeriod`, `GetAirControlFactor`, `GetAirControlVelocityMultiplier`, `GetJumpMultiplier`, `AutoJump`. The **new virtual functions** the player adds are `SetProfileData`, `FetchProfileData`, `DoReload`, `GetWeaponInActiveHand`, `CanBeUsedForFiring`, the left-hand choosing trio, the combo mode trio and `GetComboWeaponsModeDuration`, `SetCurrentWeapon`, `GetWeaponIndexInPlayerHand`, `GetWeaponFiringButton`, `IsWeaponFiringPressed`, the find/set weapon functions (favourite, strongest, next, previous), `OnNextWeapon`, `OnPrevWeapon`, `OnToggleLastWeapon`, `SetNewWeapon`, `SetNewWeaponToPlayerHand`, `DeleteWeapons`, `OnHudCommand`, `OnCameraStarted`, `SetWeaponFOVMultiplier` and `DumpCoordsInfo` (plus about 34 slots with no exported name). So **a puppet that is not a player has none of the weapon-in-hand and profile hooks**: they are introduced by this class.

#### 3.3 Members (`SPlayerPuppetEntity`, 94; 34 replicated)

`#` = replicated (`SMF_NETUPDATE`), `@` = owned object. Types are from the name prefix (Hungarian, `medium`): `f` float, `b` bool, `sl` signed 32-bit, `ub` unsigned byte, `ul` unsigned 32-bit, `str` string, `tm` time, `hen`/`hhe`/`hhud`/`hss`/`htq` handle to an entity or object, `p..` pointer to a resource, `v` vector, `qv` position and rotation, `col` colour, `i` integer, `dp` damage position. **Meaning is from the name** (low) unless a cell says more.

| Group | Members |
|---|---|
| weapons in hand | `en_henCurrentRightHandWeapon`, `en_henCurrentLeftHandWeapon`, `en_wiLastSelectedWeapon#`, `en_wiLastSelectedWeaponLeft#`, `en_wiPreLastSelectedWeapon#`, `en_wiLeftHandWeaponToBringUp#`, `en_wiRightHandWeaponToBringUp#`, `en_wiLastComboRightHandWeapon#`, `en_wiLastComboLeftHandWeapon#`, `en_bIsDualWielding#`, `en_bChoosingLeftWeapon`, `en_bLastFrameChoosingLeftWeapon`, `en_tmActivatedChoosingOfLeftWeapon`, `en_tmLastWeaponChange`, `en_bShowChangeInWeaponsList`, `en_tmLastGrenadeThrown`, `en_fWeaponFOVMultiplier`, `en_fWeapondSpeedY`, `en_fWeaponOffsetY` (the `wi` members hold a `WeaponIndex`, section 7.1) |
| combo mode | `en_fComboWeaponsTotalDuration`, `en_tmComboWeaponsStarted`, `en_bLastFrameActiveComboMode` |
| inventory and power-ups | `en_piPlayerInventory#` (the embedded `CPlayerInventory`, section 6), `en_ppumPowerUpManager@`, `en_fJumpPowerupJumpMultiplier#`, `en_fJumpPowerupAirControlPeriod#`, `en_fJumpPowerupAirControlFactor#`, `en_fJumpPowerupVelocityMultiplier#` (the four values the "serious jump" power-up substitutes, section 7.2), `en_pmmMaterialMixer@` |
| identity and score | `en_strPlayerName#`, `en_strCurrentPlayerName`, `en_strTargetedPlayerName`, `en_strCurrentCharacterModel`, `en_strMultiPlayerCharacterModel#`, `en_pmcCurrentPlayerModel`, `en_slFrags#`, `en_slDeaths#`, `en_bRenderScores`, `en_henCurrentChapter#`, `en_ulCheatFlags#` |
| preferences (copied from the profile and replicated) | `en_bAutoAiming#`, `en_bAutoChangeWeapon#`, `en_acwAutoChangeWeapon#`, `en_bViewBobbing#`, `en_bFlipFireButtons#`, `en_bPrefersThirdPersonView#`, `en_ptxCrosshairTexture#`, `en_fCrosshairSize#`, `en_colCrosshairColor#`, `en_bIsSprintingToggled#`, `en_bUseAutoJump#` |
| sprint | `en_bIsSprinting`, `en_bSprintingLastStep`, `en_tmStartedSprinting`, `en_tmStoppedSprinting`, `en_tmToggledSprint`, `en_bShowingSprintToggledNofication` |
| freezing | `en_fFreezingRatio`, `en_tmFreezingDamageApply`, `en_tmFreezingExhalation`, `en_hreExhalationEffect`, `en_pbsExhalation`, `en_pefExhalation` (the player freezes and exhales breath; the numbers are `ppp_fFreezeTime`, `ppp_fUnfrezeTime`, `ppp_fFreezingDamagePerSec`, `ppp_fFreezingExhalePeriod` in the params) |
| view feel | `en_fEyesSpeedY`, `en_fEyesOffsetY`, `en_iMoveBankingDir`, `en_fMoveBanking`, `en_fSideStepBanking` |
| damage cues | `en_sbLastDamageH#`, `en_ubDamageCountNet#`, `en_ubDamageCountLocal`, `en_dpDamagePos#`, `en_slLastAppliedDamage#` |
| lives and misc. | `en_tmLifeAdded`, `en_bLifeAddedByScore`, `en_bFirstStep`, `en_slCustomData`, `en_tmLastRiding`, `en_tmWoundSound` |
| HUD, text and speech objects | `en_htqTextQueue@`, `en_hhudHeadUpDisplay@`, `en_hheConversation`, `en_hheNetricsaIcon`, `en_hheFragsCounter`, `en_hheItemPicked`, `en_hheExtraLife`, `en_hhePlayerDeath`, `en_hheEnemyPurged`, `en_hssPlayerQuotes@`, `en_hssNetricsaQuotes@`, `en_pfntBig`, `en_ptfxcfgNotification`, `en_ptfxcfgChangeLeftWeapon`, `en_ppamAchievementManager@` |

Flag counts (34 replicated, 7 owned, 0 temporary of 94) agree with `replication.md` (**34 / 94**); that spec lists the replicated ones by role. The member **count of 94** is the table of this build; the data has no copy to compare with (the class is not saved).

### 4. `CPlayerBrainEntity` (the player's controller)

**Role.** The brain that turns a player's input (the `ClientAction` message of `replication.md`) into the puppet's actions: movement and look input, the fire buttons, the view ray for auto-aim, "use", weapon switching, respawn and the chapter boot. **Net:** `NEB_PREDICTED`; relevance distance unlimited; **medium**. **Parent:** `CBrainEntity` (constructor and 192 of 211 vtable slots unchanged). The other brains (`CAIBrain` and the vehicle and enemy kinds) are T01-104; the input methods exist only on this class among the brains of the family (name).

**Exported methods (76 symbols, 69 named), grouped.**

| Group | Methods | What they do |
|---|---|---|
| life cycle and net | `OnCreate`, `OnCreatedByPuppet`, `OnBoot`, `OnDelete`, `OnSpawn`, `OnStep`, `PreWorldChange`, `PostWorldChange`, `CollectForSortedIteration`, `GetNetBehavior`, `GetUpdatePriority`, `GetRelevantDistance`, `PreSendUpdate`, `PostReceiveUpdate`, `AdjustBaseline` | entity hooks and the net hooks (`AdjustBaseline` is called on the replicated baseline of the predicted brain: T01-098) (name) |
| input | `ClientAction`, `ProcessPlayerControls`, `ClampLookDirEul`, `GetLookDirEul`, `StartFire`, `StopFire`, `Reload`, `ForceReload`, `OnNextWeapon`, `OnPrevWeapon`, `OnToggleLastWeapon`, `OnUse`, `GetUsageDistance`, `OnShowWayPoint`, `SpreadOperatorOwnership`, `DoYouOperateMe` | the per-frame input, the fire and reload commands, weapon switching, "use" with its reach (the global `ggp_fUsageDistance` of 4 plus, when riding, a term proportional to the distance between the ride's two points; section 7.2), and which machine operates the brain (name; `ClientAction` is T01-098) |
| aiming | `ProcessAutoAiming`, `ProcessViewRayCasting`, `ProbeTargetedPlayer`, `IsAutoAimingSupported` | auto-aim, the view ray (`ggp_fRayFromViewDistance`), and the name tag of the player in the crosshair (name) |
| riding | `TryFindBestRideAndSeat`, `ShouldTeleportToEnterRide`, `GetGroupMover` | choose a vehicle and seat to enter (name) |
| respawn and chapter | `DoRespawn`, `ExecuteRespawn`, `BootPlayerOnChapter`, `BootPlayerInDeathmatch`, `GotoCurrentChapter`, `StoreChapterSaveData`, `AddWeaponsFromPastChapters`, `ReceiveScore`, `OnIKilledPuppet`, `ResetCheating`, `GetPuppetName`, `IsInvulnerable`, `HasFoeManager`, `DisablesForceImpulses`, `RenderView`, `DisplayChatMessage`, `ProcessChatMessage` | start the player in a chapter (`BootPlayerOnChapter(chapter boot type)`, `save-game.md`), store the player into the chapter save, give the weapons of past chapters, award score, chat (name; `DoRespawn` is a client-to-server call) |
| cheats | `OnGodCheat`, `OnFlyCheat`, `OnGhostCheat`, `OnGiveAllCheat`, `OnHealthCheat`, `OnArmorCheat`, `OnInvisibleCheat`, `OnAutoAimingCheat`, `OnAddLifeCheat`, `OnAddLivesCheat`, `OnSetLivesCheat`, `OnToggle3rdPersonViewCheat`, `OnTurboCheat`, `UsesGodCheat` | the 13 cheat requests of the player (13 of the 15 client-to-server cheat names of `replication.md`; `OnKillAllCheat` and `OnSpawnEntityCheat` are on the game info entity, which also has the world-wide ones) and `UsesGodCheat` (whether the player has used the god cheat, name) |

**Virtual functions overridden** (vtable against `CBrainEntity`: 19 overridden, 16 new): the exported ones are `mdGetDataType`, `OnCreate`, `OnDelete`, `OnStep`, `PostWorldChange`, `CollectForSortedIteration`, `GetRelevantDistance`, `PreSendUpdate`, `AdjustBaseline`, `PostReceiveUpdate`, `OnBoot`, `OnCreatedByPuppet`, and the **new** ones `StartFire`, `StopFire`, `Reload`, `OnPrevWeapon`, `OnNextWeapon`, `OnUse`, `TryFindBestRideAndSeat`, `RenderView`, `GotoCurrentChapter`, `StoreChapterSaveData`, `BootPlayerOnChapter`, `ReceiveScore`, `DoRespawn`, `ExecuteRespawn`, `ProcessPlayerControls`, `ClientAction` (**medium**).

**Members** (`SPlayerBrainEntity`, 31; 17 replicated; flags agree with `replication.md` 17 / 31): input `en_vOperatorLookDirEul`, `en_vOperatorRelMove`, `en_ubFireButtonStates`, `en_ubMustReleaseButtons`, the network copies `en_vNetOperatorLookDirEul#`, `en_vNetOperatorRelMove#`, `en_ubNetFireButtonStates#`; reset requests `en_ubResetLookDir#`, `en_ubLastAppliedResetLookDir`, `en_ubResetPosition#`, `en_ubLastAppliedResetPosition`; prediction `en_vPredictedVelocityCorrection#`, `en_vServerLastPosition#`, `en_henServerLastFloor#`, `en_henServerLastMover#`, `en_tmFirstReceivedClientActionAfterBoot`; aim `en_henAutoAimed#`, `en_henLastAutoAimed`, `en_vHitPointFromView`, `en_fHitDistanceFromView#`, `en_hhlHitHullFromView`, `en_tmFoundNewAutoAimedEntity`, `en_henEntityHitWithViewRay#`, `en_bEntityHitWithViewRay#`, `en_henUsableEntity#`; identity `en_iPlayer#`, `en_strPlayerName#`; misc. `en_tmLastPressedWayPoint`, `en_fTurboMultiplier`, `en_tmStartedLiving` (temporary), `en_bInvulnerablePeriodOnBootActive#`. Meaning from names (low); the prediction roles are `replication.md` 3.2.

#### 4.1 Auto-aim (T01-221: `ProcessAutoAiming`, `ProcessViewRayCasting`, read in full)

**How `en_henAutoAimed` is chosen.** Every call to `ProcessAutoAiming`: for every other puppet in the world (not the viewer, must have a mechanism, must not share the viewer's own brain) it reads a **capture-angle graph** from the global game parameters (`CGraphHolder` at a fixed offset of `_sam_pggpGlobalGameParams`), evaluated at the candidate's distance from the viewer and scaled by a per-session multiplier from the game info entity, turns that angle into a radius via `tan`, **inflates the candidate's bounding box by that radius**, and tests the player's view ray against the inflated box (`mthIntersectRayBox`). Every candidate the ray passes through is collected with its distance; the whole list is then **sorted** (`qsort`) and walked in that order, and the brain assigns `en_henAutoAimed` to the **first** candidate whose line of sight is actually clear (a fresh ray cast that either hits nothing or hits exactly that candidate) — so the effective rule is "nearest candidate inside its own distance-scaled capture cone that is not hidden behind something else."

**How it is released.** There is no separate release call: `ProcessViewRayCasting` **unconditionally resets `en_henAutoAimed` (and three other view-related handles: the view-ray hit entity and its flag, and the usable entity) to null at the start of every one of its own calls**, before `ProcessAutoAiming` (called after it, per frame) has a chance to set it again. So a target is "released" simply by not being the nearest-clear candidate on a later frame, not by an explicit clear.

**`IsAutoAimingSupported` with the weapons in hand**, confirmed: beyond the `en_bAutoAiming`/game-type gate (table above), when the player is **not riding**, the function reads **each hand's weapon object directly** (not a virtual call) at the offset of `en_bSupportAutoAiming` (`weapon-classes.md`) and **ORs the two flags together** — either hand having the flag is enough. When the player **is riding** a vehicle with a mounted weapon, that single mounted weapon's own flag (read at a different offset on the mount) replaces the hand-weapon check entirely.

**The view target the shot is bent towards.** `ProcessViewRayCasting` separately casts the player's own straight-ahead view ray (`ggp_fRayFromViewDistance` of the global parameters, as the summary already notes) and fills `en_henEntityHitWithViewRay`, `en_vHitPointFromView`, `en_fHitDistanceFromView` and `en_hhlHitHullFromView` from it; `en_henUsableEntity` is set from the same ray when the hit is within the usage reach and the entity's own "can be used" question answers yes. **Which of `en_henAutoAimed` or this straight-ahead hit a weapon's own fire function actually bends the shot towards was not traced to a specific weapon in this reading** — the consumer side is deferred to `camera-view.md`/T01-121 as the existing open question already said.

### 5. The session: `CGameInfoEntity` and `CVotingInfoEntity`

#### 5.1 `CGameInfoEntity` (80 symbols, 72 named; 8 members)

**Role.** The single entity of a running world that holds the session: the up to **17 player slots**, the statistics, the session parameters, the stored level status, the winner and the round end. `GetPlayersCount` counts the non-empty slots, `GetCountOfActivePlayers` counts the slots whose entity is a player puppet (both loops run over the 17 slots, **medium**). **Net:** `NEB_DYNAMIC`, relevance distance unlimited. **Parent:** `CBaseEntity` (its constructor calls the `CBaseEntity` one; the member table names the S layer `SBaseEntity`, section 1).

| Group | Methods | What they do |
|---|---|---|
| life cycle | `OnCreate`, `OnSpawn`, `OnBoot`, `OnShutDown`, `OnStep`, `PreWorldChange`, `PostWorldChange`, `UpdateTemps`, `PostReceiveUpdate`, `GetNetBehavior`, `GetRelevantDistance` | entity hooks (name) |
| players | `CreatePlayer`, `AttachPlayer`, `PlayerAdd`, `PlayerRemove`, `CollectPlayers`, `GetPlayer`, `GetPlayerBrain`, `GetPlayerPuppet`, `GetPlayersCount`, `GetCountOfActivePlayers`, `GetAllPlayersInRange` | add, remove and look up players (the remote calls `CreatePlayer`, `AttachPlayer`, `PlayerRemove` are in `replication.md`) |
| scoring and winner | `ReceiveScore`, `CompareScores`, `GetSortedScoreList`, `FindBestPlayer`, `DeclareWinner`, `IsWinnerDeclared`, `GetRemainingTime`, `GetTimeSinceRoundEnd`, `SpreadCountOfLeftFragsChanged`, `SpreadPlayerDeathInfo`, `CalculateGameTotalStats` | rank players and end a round (rules read in section 7.4) |
| session rules | `GetEnemyMultiplier`, `IsItemAllowed`, `IsComboWeaponsEnabled`, `IsRocketJumpingEnabled`, `IsCheatingEnabled`, `HasEverCheated`, `UpdateGameDifficultyParams`, `GetGameDescription`, `GetDescription`, `GetCurrentLevelName`, `NextScreenSize` | the options of the session as the game asks for them (rules in section 7.4) |
| chapter save | `GenerateChapterSaveGame`, `StorePlayersData`, `SetLevelStatus`, `DisableChapterResurrection`, `DumpChapterSavePlayers`, `RestoreSavedSecrets`, `RestoreSavedObjectivesAndNavBeacons`, `SpreadChapterFinishedEvent`, `SpreadLevelFinishedEvent` | build the chapter save from the live players and apply it on restart (`save-game.md` section 6) |
| messages | `SpreadSpeechNotificaton`, `SpreadShowNetricsaIcon`, `SpreadHideNetricsaIcon`, `AwardAchievement`, `SetAchievementProgress`, `GetAchievementsStatus` | tell all players; achievements (name) |
| cheats | `OnGodCheat`, `OnFlyCheat`, `OnGhostCheat`, `OnGiveAllCheat`, `OnHealthCheat`, `OnArmorCheat`, `OnInvisibleCheat`, `OnKillAllCheat`, `OnSpawnEntityCheat`, `OnTurboCheat`, `OnAddLife`, `OnAddLivesCheat`, `OnSetLivesCheat` | the server side of the cheats that act on all players or the world (name) |

**Virtual functions overridden** against `SBaseEntity` (12 overridden, 20 new): `mdGetDataType`, `OnCreate`, `OnStep`, `GetRelevantDistance`, `PostReceiveUpdate`, `OnBoot`, `UpdateTemps` and the new `GetGameDescription`, `DisableChapterResurrection`, `SetLevelStatus`, `GetPlayerBrain`, `GetPlayerPuppet`, `CreatePlayer`, `AttachPlayer`, `PlayerAdd`, `PlayerRemove`, `CollectPlayers`, `GenerateChapterSaveGame`, `StorePlayersData`, `SpreadSpeechNotificaton`, `SpreadShowNetricsaIcon`, `SpreadHideNetricsaIcon`, `GetPlayer`, `DumpChapterSavePlayers`, `GetDescription`, `NextScreenSize` (**medium**).

**Members** (`SGameInfoEntity`, 8; 4 replicated): `en_ahenPlayers` (array of the 17 player handles), `en_gsGameStats#` (an embedded `CGameStats`, section 8), `en_sgpGameParams#` (the session parameter block; its 28 replicated members are listed in `replication.md`; the class is exported as `CSamProjectParams` and its member table is named `CSamTestRunParams`: T01-113), `pc_ssScreenSize#`, `en_hslsStoredLevelStatus@` (the stored level status, section 8), `en_henVotingInfoEntity@` (the voting entity), `en_iWinnerIndex#` (the slot of the winner, -1 = none), `en_tmRoundEnd`.

#### 5.2 `CVotingInfoEntity` (26 symbols, 19 named)

A voting entity owned by the game info entity: `StartVote`, `VoteYes`, `VoteNo`, `RegisterVote`, `VoteReset`, `VotePass`, `VoteFail`, `CheckVoteStatus`, `IsVoteInProgress`, `IsVoteTypeEnabled`, `HasVotePassed`, `HasVoteFailed` and the entity hooks (`OnCreate`, `OnStep`, `PostWorldChange`, `PreWorldChange`, `PostReceiveUpdate`, `GetNetBehavior` = `NEB_DYNAMIC`, `GetRelevantDistance` = unlimited). `StartVote`, `VoteYes`, `VoteNo` are the client-to-server calls of `replication.md`. Rules read: a vote **passes when more than half of the players (the count of non-empty slots) have voted yes and fails when more than half have voted no** (the threshold is the constant 0.5, "more than", **medium**); `IsVoteTypeEnabled` compares the vote type with a list of names including `all`, `nextmap` and `restart` (a server setting; the full list and its variable are T01-101).

#### 5.3 The two HUD elements

`CPlayerDeathHudElement` (`AddMessage`, `Clear`, `PrepareRenderingParameters`, `Render`) draws the death messages; `CVotingInfoHudElement` (`PrepareRenderingParameters`, `Render`) draws the running vote. Their parameter classes are saved (`CPlayerDeathHudElementParams`, `CVotingInfoHudElementParams`, section 9). The rest of the HUD is T01-111.

### 6. The inventory

#### 6.1 `CInventoryBase` and `CPlayerInventory`

`CInventoryBase` has no named method (6 symbols). `CPlayerInventory` (30 symbols, 24 named) is the player's belongings: **lives, the set of owned weapons, the set of weapons held doubled, the ammunition counts and the power-up end times**. It is a **member of the player puppet** (`en_piPlayerInventory`, replicated), constructed by the puppet's constructor, not an entity.

Members (five; the table calls them `pi_...`; flags from the table): `pi_ctLives#` (lives), `pi_ulWeapons#` (bit set of owned weapons, bit *n* = `WeaponIndex` *n*), `pi_ulDoubleWeapons#` (bit set of weapons held in two hands), `pi_aslAmmoQuantity#` (one signed 32-bit count per ammunition kind), `pi_atmPowerUp` (one end time per power-up, not replicated: a client learns power-ups through the puppet's own members and the `ReceivePowerUp` call).

| Methods | What they do (read in section 7.3) |
|---|---|
| `ReceiveWeapon`, `RemoveWeapon`, `ClearWeapons`, `GetCountOfPossessedWeapons`, `IsWeaponInInventory`, `IsWeaponDoubled`, `CanBeUsedForFiring` | set, remove and test weapon bits; a weapon can fire when it is owned and the ammunition covers one shot |
| `ReceiveAmmo`, `DrainAmmo`, `DrainAmmoFromWeapon`, `ClearAmmo`, `GetAmmoQuantity`, `GetAmmoQuantityForWeapon`, `GetMaxAmmoQuantity`, `GetAmmoRatioForWeapon`, `AssureAmmoRatio`, `AssureAmmoRatioForWeapon`, `AssureAmmoRatioForAllPossessedWeapons`, `SetAmmoValues`, `SetAmmoFromChapter` | add, spend and query ammunition; "assure a ratio" lifts the count to at least a fraction of the cap; the last two set the start ammunition from a chapter (the chapter's ammunition container and ratio) |
| `ReceivePowerUp`, `ResetPowerUps`, `ResetAllToDefault`, `GetDescription` | power-up end times (a received power-up keeps the later of the old and the new end time), reset to the default state, dump to the log |

#### 6.2 The tables behind the inventory

See section 7.3 for the three tables (cap, cost, kind per weapon). The inventory itself stores only counts and bits.

#### 6.3 The obsolete `*InventoryItem` classes

26 classes: `CBaseInventoryItemObsolete` (the name says "obsolete"), its two children `CWeaponInventoryItem` and `CAmmoInventoryItem`, and one class per kind: **13 weapons** (`CAutoShotgun`, `CBeamGun`, `CCannon`, `CCircularSaw`, `CDoubleShotgun`, `CGrenadeLauncher`, `CKlodovikGun`, `CMiniGun`, `CPlasmaRifle`, `CRocketLauncher`, `CSeriousBomb`, `CUzi`, `CZapGun` + `InventoryItem`) and **10 ammunition kinds** (`CBlades`, `CBullets`, `CCannonBalls`, `CGrenades`, `CHandGrenades`, `CKlodovik`, `CPlasma`, `CRockets`, `CSerious_Bomb`, `CShells` + `InventoryItem`). Each exports only the standard 7 symbols (no member table). **Nothing constructs them:** the only callers of their constructors are the constructor of the next class up and two small functions per class (the registration of the type, **medium**). They cover 13 of the 17 weapon indices (no Colt, Sniper, Combo, Flamer) and 10 of the 12 ammunition kinds (no sniper bullets, napalm): an older item model that was left in the DLL while the inventory moved to bit sets and counts (**an interpretation**, low).

### 7. Enumerations, tables and rules read from the binary

#### 7.1 Enumerations (name and value as the binary declares them)

| Enumeration | Values |
|---|---|
| `WeaponIndex` (17; -1 `WI_NONE`) | 0 `WI_CIRCULAR_SAW`, 1 `WI_ZAP_GUN`, 2 `WI_AUTO_SHOTGUN`, 3 `WI_DOUBLE_SHOTGUN`, 4 `WI_UZI`, 5 `WI_MINI_GUN`, 6 `WI_ROCKET_LAUNCHER`, 7 `WI_GRENADE_LAUNCHER`, 8 `WI_PLASMA_RIFLE`, 9 `WI_KLODOVIK_GUN`, 10 `WI_CANNON`, 11 `WI_SERIOUS_BOMB`, 12 `WI_COLT`, 13 `WI_SNIPER`, 14 `WI_COMBO_WEAPONS`, 15 `WI_BEAM_GUN`, 16 `WI_FLAMER` |
| `AmmoIndex` (12; -1 `AI_NONE`) | 0 `AI_BLADES`, 1 `AI_BULLETS`, 2 `AI_CANNON_BALLS`, 3 `AI_GRENADES`, 4 `AI_HAND_GRENADES`, 5 `AI_KLODOVIK`, 6 `AI_PLASMA`, 7 `AI_ROCKETS`, 8 `AI_SERIOUS_BOMB`, 9 `AI_SHELLS`, 10 `AI_SNIPER_BULLETS`, 11 `AI_NAPALM` |
| `PowerUpIndex` (7; -1 `PUI_NONE`) | 0 `PUI_SERIOUS_STRENGTH`, 1 `PUI_SERIOUS_SPEED`, 2 `PUI_SERIOUS_JUMP`, 3 `PUI_SERIOUS_DAMAGE`, 4 `PUI_SERIOUS_SCORE`, 5 `PUI_INVULNERABILITY`, 6 `PUI_INVISIBILITY` |
| game type (bit flags) | 0 `GT_NONE`, 1 `GT_SINGLEPLAYER`, 2 `GT_COOPERATIVE`, 4 `GT_DEATHMATCH`, 8 `GT_TEAMDEATHMATCH`, 16 `GT_CAPTURETHEFLAG` |
| difficulty | 0 `GD_NONE`, 1 `GD_TOURIST`, 2 `GD_EASY`, 3 `GD_NORMAL`, 4 `GD_HARD`, 5 `GD_SERIOUS` |
| chapter boot type | 0 `CBT_CUSTOM_LEVEL_CUSTOM_CHAPTER`, 1 `CBT_MAIN_MENU_CONTINUE_PLAYING`, 2 `CBT_RESPAWN_AFTER_DEATH`, 3 `CBT_INGAME_MENU_RESTART_CHAPTER`, 4 `CBT_GAME_OVER_MENU_CONTINUE_PLAYING`, 5 `CBT_GAME_OVER_MENU_RESTART_LEVEL`, 6 `CBT_COOP_RESPAWN_PLAYER`, 7 `CBT_COOP_RESTART_LEVEL`, 8 `CBT_COOP_JOIN`, 9 `CBT_DEVELOPER_CUSTOM_CHAPTER`, 10 `CBT_LEVEL_CHANGED`, 11 `CBT_BOOT_TEMPLATES` |

**Cross-checks with the data (medium):** (a) the seven members of `CPowerUpDurations` (`pud_fSeriousStrength`, `...Speed`, `...Jump`, `...Damage`, `...Score`, `pud_fInvulnerability`, `pud_fInvisibility`) are in exactly the order of `PowerUpIndex`; (b) the "give all weapons" mask of `player-rules.md` is `0xBFFF`: bits 0 to 13 and 15 set, bit 14 (`WI_COMBO_WEAPONS`) clear, bit 16 (`WI_FLAMER`) outside the mask, which fits a combo pseudo-weapon that is never given and a flamer added after the mask was set; (c) `CGameDifficulties` has five members, `gd_dpTourist` to `gd_dpSerious`, the five named levels. The chapter boot type names answer the "no value names" remark of `save-game.md`: the types 0, 1, 3, 5, 7, 9, 10 that restore secrets are *custom level*, *continue from the main menu*, *restart chapter from the in-game menu*, *restart level from the game-over menu*, *co-op restart level*, *developer chapter* and *level changed*; the types 1 to 4 that restore objectives and beacons are *continue*, *respawn after death*, *restart chapter* and *continue from game over*. (Mapping by value; **medium**.)

#### 7.2 Rules read from small functions of the player classes

All **medium** (one reading each) unless stated; the numbers are what the function returns.

| Rule | Value | Notes |
|---|---|---|
| Initial maximum health of a player | **200** | `GetInitialMaxHealth`; the `.ep` files of the player say nominal health 100 (`puppet-stats.md`) and `GetNominalHealth` returns **100** as well: two readings agree on 100 as the nominal and 200 as the ceiling |
| Damage multiplier of "serious damage" (`PUI_SERIOUS_DAMAGE`) | **4.0** in single player and co-op, **2.5** in a deathmatch-type session; 1.0 without the power-up | `GetApplyDamageMultiplier`; the help file's "x4" of `player-rules.md` is the single-player value |
| Damage taken multiplier | in a non-deathmatch session: the value the game info entity caches from the difficulty (`dif_fPlayerDamageMultiplier`, `UpdateGameDifficultyParams`); in deathmatch: **1.0** | `GetReceiveDamageMultiplier` |
| Falling damage | not received while "serious jump" (`PUI_SERIOUS_JUMP`) is active | `CanReceiveFallingDamage` |
| Jump and air control | with "serious jump": the four `en_fJumpPowerup...` members (replicated) replace the jump multiplier, the air-control period and factor and the air-control velocity multiplier; without it: the world info's two air-control members (`CWorldInfoEntity`) if they are not negative, else the legged puppet's defaults | `GetJumpMultiplier`, `GetAirControl...` |
| Speed | the base legged speed; times the `ppp_fSpeedPowerUpMultiplier` of the params (2.0 if the params are missing) while `PUI_SERIOUS_SPEED` is active; times **1.5** while **sprinting** (sprint enabled by the session option `sgp_bEnableSprinting`, `en_bIsSprinting` set, and `en_ppPuppetPose` not equal to 2; the pose names are not in the data, 2 is probably crouching: **low**); a further factor **1.3** in deathmatch under a condition not resolved | `GetSpeedMultiplier` (the 1.3 part is **low**) |
| Invulnerability | invulnerable while `PUI_INVULNERABILITY` is active **or for 1.0 s after `en_tmLastRiding`** (the last time the player was riding, so a player who gets off a vehicle is protected for one second); otherwise the parent's rule | `IsInvulnerable` (the member was identified by the offset match between the code and the member table: **medium-low**) |
| Auto-aim | supported only when the player's `en_bAutoAiming` is on (confirmed at the puppet's own offset by `OnAutoAiming`'s toggle) **and** the session is single player or co-op (`GT_SINGLEPLAYER`/`GT_COOPERATIVE`); **otherwise (T01-221, `IsAutoAimingSupported` read in full), when not riding: the OR of each hand's weapon's own `en_bSupportAutoAiming` flag** (`weapon-classes.md`'s named member, read directly from the weapon object, not asked for virtually) **— either hand qualifying is enough; when riding a vehicle with a mounted weapon, that mounted weapon's own flag (at a different, separately-read offset) is used instead of the hand weapons** | `CPlayerBrainEntity::IsAutoAimingSupported`, read in full (T01-221) |
| Kill score of a player | the puppet's normal kill score; **100000** in a deathmatch-type session | `GetKillScore`; **low** (the unit of "score" in deathmatch is not clear, section "Open questions") |
| Corpse stays | the value **-1** (a "no limit" marker, **low**) | `GetCorpseStadyPeriod` |
| Net | update priority **10.0**; relevance distance 3.0e38 (always relevant) for the puppet, the brain, the game info and the voting entity | one-line functions |
| Usage reach | the global `ggp_fUsageDistance` (4) plus, while riding, 1.25 times the distance between two points of the ride | `CPlayerBrainEntity::GetUsageDistance` (the 1.25 factor is **medium-low**) |

#### 7.3 Ammunition and weapon tables (the inventory)

Three tables in the DLL, indexed by the enumerations of 7.1 (**medium**: one reading, plus the consistency checks below).

| Weapon (index) | Ammunition kind used | Ammunition used per shot |
|---|---|---|
| `WI_CIRCULAR_SAW` (0) | none | 0 |
| `WI_ZAP_GUN` (1) | none | 0 |
| `WI_AUTO_SHOTGUN` (2) | `AI_SHELLS` | 1 |
| `WI_DOUBLE_SHOTGUN` (3) | `AI_SHELLS` | 2 |
| `WI_UZI` (4) | `AI_BULLETS` | 1 |
| `WI_MINI_GUN` (5) | `AI_BULLETS` | 1 |
| `WI_ROCKET_LAUNCHER` (6) | `AI_ROCKETS` | 1 |
| `WI_GRENADE_LAUNCHER` (7) | `AI_GRENADES` | 1 |
| `WI_PLASMA_RIFLE` (8) | `AI_PLASMA` | 1 |
| `WI_KLODOVIK_GUN` (9) | `AI_KLODOVIK` | 1 |
| `WI_CANNON` (10) | `AI_CANNON_BALLS` | 1 |
| `WI_SERIOUS_BOMB` (11) | `AI_SERIOUS_BOMB` | 1 |
| `WI_COLT` (12) | none | 0 |
| `WI_SNIPER` (13) | `AI_SNIPER_BULLETS` | 1 |
| `WI_COMBO_WEAPONS` (14) | none | 0 |
| `WI_BEAM_GUN` (15) | `AI_PLASMA` | 1 |
| `WI_FLAMER` (16) | `AI_NAPALM` | 1 |

| Ammunition kind | Cap |
|---|---|
| `AI_BLADES` 50 · `AI_BULLETS` 1000 · `AI_CANNON_BALLS` 30 · `AI_GRENADES` 50 · `AI_HAND_GRENADES` 30 · `AI_KLODOVIK` 12 · `AI_PLASMA` 500 · `AI_ROCKETS` 50 · `AI_SERIOUS_BOMB` 3 · `AI_SHELLS` 200 · `AI_SNIPER_BULLETS` 50 · `AI_NAPALM` 500 | the base cap of each kind (single player, enemy multiplier 1) |

Rules (all **medium**):

- **Cap and pickups scale with the enemy multiplier.** The cap used everywhere is the table value times `GetEnemyMultiplier`, and `ReceiveAmmo` adds the amount times the same factor, then clamps the count to the range from 0 to the cap. `GetEnemyMultiplier` is **1** unless the game type is single player or co-op; then it is the session option `sgp_iEnemyMultiplier` (at least 1); in a co-op session the function adds a further, non-negative term that depends on the number of active players (**the term is not resolved**, Open question). So more enemies mean larger caps and larger pickups.
- **Infinite ammunition.** A weapon's ammunition kind is the table value unless the session option `sgp_bInfiniteAmmo` is set, in which case it is "none": a weapon with no kind needs no ammunition (`CanBeUsedForFiring`: owned, and the kind is none or the count is at least the cost).
- **Weapons that never need ammunition:** saw, zap gun, Colt, combo pseudo-weapon. **Ammunition without a weapon in the table:** `AI_BLADES` and `AI_HAND_GRENADES` (used by the thrown saw blades and the hand grenade, which are not weapon indices; **low**).
- **`ReceiveWeapon`** sets the weapon's bit; for `WI_SERIOUS_BOMB` it clears the owned and doubled bits instead when the session option `sgp_bDisableSeriousBombs` is set and the game type is co-op.
- **Defaults** (`ResetAllToDefault`, `ClearWeapons`): the owned set is **{saw, zap gun, Colt}** (bits 0, 1, 12; in a deathmatch-type session **{saw, Colt}**), the doubled set is **{Uzi, Colt}** (bits 4 and 12), all ammunition counts are 0, all power-up end times are set far in the past (1000 s before time zero) and the lives come from a global value (**not resolved**). `GetAmmoQuantity` of a kind outside the range returns -1; `GetMaxAmmoQuantity` of one returns 0.
- **`DrainAmmo`** (spend): fails (returns 0) when the count is below the amount; **only the host subtracts** (a client's call succeeds without changing its count).
- **`SetAmmoFromChapter`**: raises each owned weapon's ammunition to the chapter's ratio of the cap, raises the 5th kind (hand grenades) to its ratio as well, then applies the chapter's **ammunition container** (`CAmmoItemContainer`, a data class of 39 world files in `classes-catalog.md`) which sets listed kinds to listed counts.
- **`ReceivePowerUp(index, seconds)`** stores the end time now + seconds only if it is **later** than the stored one: a second pickup extends, never shortens.

#### 7.4 Session rules of the game info entity (read)

| Rule | Value | Evidence |
|---|---|---|
| Players | **17** slots | the loops of `GetPlayersCount` and `GetCountOfActivePlayers` |
| Scoring in a non-deathmatch session | `CompareScores` returns 1 (no ordering) | one reading |
| Deathmatch ranking | more frags first (`en_slFrags`), then **fewer deaths** (`en_slDeaths`); `FindBestPlayer` returns the slot of the unique best, **-1 if the best two tie on both** | `CompareScores`, `FindBestPlayer` |
| Remaining time | `sgp_ctTimeLimit` minutes times 60, minus the elapsed seconds | `GetRemainingTime` |
| Winner declared | `en_iWinnerIndex` is not -1 | `IsWinnerDeclared` |
| Combo weapons | enabled only if the global setting `ggp_cwuComboWeaponsUsing` is 2, the session option `sgp_bUseComboWeapons` is set and the game type is single player or co-op | `IsComboWeaponsEnabled` |
| Rocket jumping | `sgp_bEnableRocketJumping` and single player or co-op | `IsRocketJumpingEnabled` |
| Item filter | in a deathmatch-type session on the host or locally: an item is allowed unless its type is in the session's disabled list | `IsItemAllowed` (the list's origin: T01-101) |
| Enemy multiplier | as in 7.3 | `GetEnemyMultiplier` |

### 8. Save and statistics records

| Class | Role | Members (table, prefix) | Exported methods |
|---|---|---|---|
| `CPlayerSave` | one saved player in the chapter save | `ps_strPlayerName` (default text "Serious Sam"), `ps_ulWeapons`, `ps_ulDoubledWeapons`, `ps_wiLastSelectedWeapon` (default -1), `ps_aslAmmoQuantity` (**11** counts, all 0 by default), `ps_slHealth`, `ps_slArmor`, `ps_ctLives` | `GetDescription` |
| `CStoredChapterStatus` | the status at one chapter | `scs_strWorld`, `scs_ulChapterEntityID`, `scs_gsGameStats` (a `CGameStats`), `scs_ahpsPlayerData` (up to 64 `CPlayerSave`; owned), `scs_ceidUnableToResurrect`, `scs_anbdNavigationalBeacons`, `scs_amodMissionObjectives`, `scs_asedSecretEntities` | `GetSavedPlayer` (by name), `GetDescription` |
| `CStoredLevelStatus` | the status of a level: current and first chapter plus the session parameters | `sls_scsFirstChapter`, `sls_scsCurrentChapter`, `sls_sgpGameParams` | none (the constructor builds two `CStoredChapterStatus`) |
| `CGameStats` | the statistics of a session | `gs_ctSecrets#`, `gs_ctSecretsTotal#`, `gs_ctKills#`, `gs_ctKillsTotal#`, `gs_slScore#`, `gs_tmPlayingTime`, `gs_slPlayingTime#`, `gs_bCheating#`, `gs_bSaving#`, `gs_bFrozenStatistic#` (9 of 10 replicated) | `Reset`, `GetDescription` |

The roles are those of `save-game.md` section 6; this table adds the **class names to the member groups** (by prefix, **medium**) and that the **saved ammunition array has 11 entries while the game has 12 ammunition kinds** (the constructor of `CPlayerSave` clears 11; section "Open questions"). `CPlayerSave` is built in `StorePlayersData` only; `CStoredLevelStatus` in `GenerateChapterSaveGame` only; `CGameStats` by the constructors of `CStoredChapterStatus` and `CGameInfoEntity` (**medium**).

### 9. Classes of the family that are in the data (reproduced by the script)

`players_session_classes_verify.py` prints this table (own members of the most common definition, number of files, extensions):

| Exported as | Saved as | Base | Own members | Files | Where |
|---|---|---|---|---|---|
| `CPlayerPuppetParams` | `SPlayerPuppetParams` | `SLeggedPuppetParams` | 23 (3 variants: 22, 23, 24) | 4 | `.ep`: `EntityParams/PlayerPuppet.ep`, `EntityParams/PlayerPuppet_DM.ep`, `Puppets/NPCs_cutsequences/Santa.ep`, `Puppets/Player/Player.ep` |
| `CCameras` | `SCameras` | none | 4 | 126 | a sub-object of the `.ep` files (`pp_cCameras`; the player's own camera set in `PlayerPuppet.ep` among them) |
| `CKnockOutParams` | `SKnockOutParams` | none | 3 | 86 | sub-object of `.ep` files |
| `CGlobalPlayerParams` | same | `CResource` | 3 (`gpp_fFOV`, `gpp_bAutoJumpSP`, `gpp_bAutoJumpDM`) | 1 | `Databases/GlobalGameParameters.rsc` |
| `CDifficultyParams`, `CGameDifficulties` | same | none | 7, 5 | 1 each | `GlobalGameParameters.rsc` |
| `CPowerUpDurations` | same | none | 7 | 1 | `GlobalGameParameters.rsc` |
| `CPlayerPuppetCrosshairs` | same | `CResource` | 20 (`ppc_ptxCrosshair00` to `19`) | 1 | `Databases/PlayerCrosshairs.rsc` |
| `CPlayerDeathHudElementParams`, `CVotingInfoHudElementParams` | same | `SHudElementParams` | 4 (number-only names), 1 | 1 each | `Databases/HudParams.rsc` |

**The 42 other classes are absent from every type table** (42 of 42, own name and `S` twin; script). **Names against the binary's table for `SPlayerPuppetParams`:** the binary declares 24 members; the data files have 22, 23 or 24; **five of the data's number-only members (`.1` to `.5` in file order) sit exactly where the binary's table declares `ppp_fMinBombThrowVelocity`, `ppp_fMaxBombThrowVelocity`, `ppp_fMinBombThrowPitchDeg`, `ppp_fMaxBombThrowPitchDeg` and `ppp_fHandGreandeRecoil`**, and every named data member is in the binary's table in the same order (script: "named members in the binary's order: True"). The script shows the same in all three variants: the first two numbered members lie between named neighbours where the binary declares exactly two names (min and max throw velocity) and the last three where it declares exactly three (min and max throw pitch, recoil). Taken in order, `.1` and `.2` are the minimum and maximum hand-grenade throw velocity, `.3`, `.4`, `.5` the minimum and maximum throw pitch in degrees and the recoil (**high** that the order and counts fit, **medium** for the one-to-one identification, as the data files carry only numbers).

## Invariants

1. 52 exported classes; 10 are in the data under the names of section 9, 42 are in no type table (script: 42 of 42). A reader must not look for the entity or inventory classes of this family in a world or `.ep` file.
2. The player is `CPlayerPuppetEntity` (net behaviour predicted), controlled by `CPlayerBrainEntity` (predicted); the session is `CGameInfoEntity` and the vote `CVotingInfoEntity` (both dynamic). A session has at most **17** player slots (read twice, `GetPlayersCount` and `GetCountOfActivePlayers`).
3. A weapon index is 0 to 16, an ammunition index 0 to 11, a power-up index 0 to 6; -1 is "none" in each; a weapon or power-up bit set uses bit *n* for index *n*.
4. A weapon needs ammunition only if the table of section 7.3 names a kind and the session option for infinite ammunition is off.
5. The ammunition caps of section 7.3 are for an enemy multiplier of 1.
6. The inventory is part of the player puppet, not an entity; the 26 `*InventoryItem` classes are not used.

## Open questions

Ranked by what blocks an implementer.

1. **The co-op term of the enemy multiplier** and whether caps and pickups really scale in co-op (the function adds a term from the number of active players; the formula, and its relation to `sgp_iEnemyMultiplierPerPlayer` and `sgp_iEnemyMaxMultiplier`, are not resolved) and the source of the starting lives. **T01-115** (read the function in full and measure a co-op session).
2. **Eleven or twelve ammunition kinds.** The saved player and the inventory declare an array of 11 counts, the game has 12 ammunition kinds (napalm is index 11): where napalm is kept, whether it is saved in a chapter save, and whether it shares storage with the first power-up time. **T01-115**.
3. **The player rules behind the methods** that were only described by name: weapon selection (favourite, strongest, next and previous, auto change), dual wielding and the combo mode, the invulnerability window after a spawn, the deathmatch speed factor of 1.3 and its condition, the deathmatch kill score of 100000 and the corpse period of -1. **T01-114** (read the functions; confirm by measurement with T01-038 and T01-039).
4. **Run-time checks that need the running game:** that the member tables, the replicated flags and the net behaviours are what a live session shows, and the identification of the five numbered `SPlayerPuppetParams` members by hand-grenade throws. **T01-116**.
5. The vote types and thresholds' configuration, the item filter list, the scoreboard and the ban commands: **T01-101**. The `ClientAction` content and the prediction: **T01-098**. The session parameter class (`CSamProjectParams` exported, `CSamTestRunParams` as the table's name) and the global parameter classes: **T01-113**. The weapon, projectile and puppet classes the player uses: **T01-103**, **T01-106**, **T01-107**. The HUD elements: **T01-111**.
6. Class sizes and vtable layouts: **T01-063** (not needed for a reimplementation).

## Test vectors

1. `WeaponIndex` 4 is `WI_UZI`; its ammunition kind is `AI_BULLETS` (1) with cap 1000 and cost 1; a bit set `0x1010` holds the Uzi and the Colt (bits 4 and 12): the default **doubled** set.
2. `WI_DOUBLE_SHOTGUN` (3) uses `AI_SHELLS` (9), cap 200, cost 2: with 3 shells it can fire once, then has 1 and cannot fire (owned, 1 < 2).
3. With the enemy multiplier 2 (single player, profile option set to 2) the cap of `AI_ROCKETS` is 100 and a pickup of 5 rockets adds 10, clamped to 100.
4. Default owned weapons in single player: bits 0, 1 and 12, that is the mask `0x1003`; in deathmatch `0x1001`.
5. A "serious damage" player deals 4 times the damage in single player and 2.5 times in deathmatch.
6. Vote of 5 players: 3 yes passes (3/5 = 0.6 > 0.5), 2 yes does not pass, and a 5-player vote with 2 yes and 3 no has failed (3/5 > 0.5); with 4 players 2 yes does **not** pass (2/4 is not more than 0.5).
7. Data (script): `SPlayerPuppetParams` is in 4 `.ep` files, `SCameras` in 126, `SKnockOutParams` in 86; the class `CPlayerPuppetEntity`, `SPlayerPuppetEntity` and `CGameInfoEntity` are in none of the 8,063 files.
