# Class inventory of `Sam2Game.dll`, table 1 of 2: `CAIBrainEntity` to `CMCPatternMoving` (T01-113)

- **Roadmap task:** T01-113 (last step of the class inventory T01-026; index, checks and gaps: [`class-inventory.md`](class-inventory.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class names, parts, function counts, data names and member table names of the 466 rows (export table, type tables, member tables; the part assignment is the T01-026 split, checked by `tools/re/class_inventory_check.py` against the pages); **medium** for the `rule` rows (the page states a family rule and names the partner class, section 3 of the index).

Prerequisites: [`class-inventory.md`](class-inventory.md) (the columns are explained in its section 3).

## Summary

466 exported classes in name order (ASCII, so `CAI...` precedes `CAdd...`); the other half is [`class-inventory-2.md`](class-inventory-2.md). Every class is in exactly one row of the two tables.

## Evidence

The table was generated from the private export tables of `Sam2Game.dll` (11,610 exports; 932 classes), the type tables of the data (`classes-catalog.md`), the DLL's member tables and the text of the pages of the twelve parts; `tools/re/class_inventory_check.py` re-checks the committed table against the pages and the part counts (it reads only the repository, no game file).

## Layout / rules

| Class | Part | Covering page | Named | In the data | Functions | Member table | Note |
|---|---|---|---|---|---|---|---|
| `CAIBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CAIBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CAIBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CAddAllWeaponsIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CAddPastChaptersWeaponsIterator` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 8 | `SAddPastChaptersWeaponsIterator` |  |
| `CAdjustWeaponMode` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 10 | - |  |
| `CAdrenalineSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CAdrenalineSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SAdrenalineSimpleHudElementParams` | 7 | - |  |
| `CAirBikeBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CAirBikeBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CAirBikeBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CAirBikePuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CAirBikePuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CAirBikePuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CAirSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CAirSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SAirSimpleHudElementParams` | 7 | - |  |
| `CAircraftBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CAircraftBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CAircraftBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CAircraftCharacterEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SAircraftCharacterEntity` | 8 | - |  |
| `CAircraftCharacterProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | S twin `SAircraftCharacterProperties` | 7 | - |  |
| `CAircraftPuppetEntity` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | name | S twin `SAircraftPuppetEntity` | 23 | `SAircraftPuppetEntity` |  |
| `CAircraftPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SAircraftPuppetParams` | 7 | `SAircraftPuppetParams` |  |
| `CAircraftPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | S twin | S twin `SAircraftPuppetProperties` | 7 | - |  |
| `CAlarmEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | - | 11 | `SAlarmEntity` |  |
| `CAlarmProperties` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | - | 8 | `SAlarmProperties` |  |
| `CAllMovingAnimation` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | same | 7 | - |  |
| `CAmmoHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 10 | - |  |
| `CAmmoHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | S twin `SAmmoHudElementParams` | 7 | `SAmmoHudElementParams` |  |
| `CAmmoIcons` | T01-111 | [`hud-classes`](hud-classes.md) | name | - | 6 | - |  |
| `CAmmoInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 7 | - |  |
| `CAmmoItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SAmmoItemEntity` | 12 | `SAmmoItemEntity` |  |
| `CAmmoItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SAmmoItemProperties` | 7 | - |  |
| `CAmmoPackItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SAmmoPackItemEntity` | 11 | `SAmmoPackItemEntity` |  |
| `CAmmoPackItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SAmmoPackItemProperties` | 7 | `SAmmoPackItemProperties` |  |
| `CAnimEventStep` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | S twin `SAnimEventStep` | 9 | `SAnimEventStep` |  |
| `CAnimatedMoverEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | S twin `SAnimatedMoverEntity` | 33 | `SAnimatedMoverEntity` |  |
| `CAnimatedMoverEnvelopeProvider` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | S twin `SAnimatedMoverEnvelopeProvider` | 14 | `SAnimatedMoverEnvelopeProvider` |  |
| `CAnimatedMoverProperties` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | S twin `SAnimatedMoverProperties` | 9 | `SAnimatedMoverProperties` |  |
| `CArmorItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SArmorItemEntity` | 13 | `SArmorItemEntity` |  |
| `CArmorItemLEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SArmorItemLEntity` | 7 | - |  |
| `CArmorItemLProperties` | T01-108 | [`item-classes`](item-classes.md) | name | S twin `SArmorItemLProperties` | 7 | - |  |
| `CArmorItemMEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SArmorItemMEntity` | 7 | - |  |
| `CArmorItemMProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SArmorItemMProperties` | 7 | - |  |
| `CArmorItemProperties` | T01-108 | [`item-classes`](item-classes.md) | name | S twin `SArmorItemProperties` | 7 | - |  |
| `CArmorItemSEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SArmorItemSEntity` | 7 | - |  |
| `CArmorItemSProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SArmorItemSProperties` | 7 | - |  |
| `CArmorItemXLEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SArmorItemXLEntity` | 7 | - |  |
| `CArmorItemXLProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SArmorItemXLProperties` | 7 | - |  |
| `CArmorItemXSEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SArmorItemXSEntity` | 7 | - |  |
| `CArmorItemXSProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SArmorItemXSProperties` | 7 | - |  |
| `CArmorItemXXLEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 7 | - |  |
| `CArmorItemXXLProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | same | 7 | - |  |
| `CArmorSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CArmorSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SArmorSimpleHudElementParams` | 7 | - |  |
| `CAssignNewProfileMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CAttachToolAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | same | 8 | - |  |
| `CAutoAimHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 9 | - |  |
| `CAutoAimHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CAutoShotgunInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CAutoShotgunItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SAutoShotgunItemEntity` | 8 | - |  |
| `CAutoShotgunItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SAutoShotgunItemProperties` | 7 | - |  |
| `CAutoShotgunWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 11 | `SAutoShotgunWeaponEntity` |  |
| `CAutoShotgunWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SAutoShotgunWeaponParams` | 7 | `SAutoShotgunWeaponParams` |  |
| `CAutoShotgunWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CAutoShotgunWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SAutoShotgunWeaponRenderingParams` | 7 | - |  |
| `CAutoTurretBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CAutoTurretBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CAutoTurretBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CAutoTurretPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CAutoTurretPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CAutoTurretPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CBAAnimation` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAAnimation` | 11 | `SBAAnimation` |  |
| `CBAChangeAligment` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | - | 9 | `SBAChangeAligment` |  |
| `CBAChangeMovingAbilities` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 8 | - |  |
| `CBAChangeState` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAChangeState` | 10 | `SBAChangeState` |  |
| `CBACrouch` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 9 | - |  |
| `CBADie` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBADie` | 9 | - |  |
| `CBAEmotion` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAEmotion` | 9 | `SBAEmotion` |  |
| `CBAFoeAssigning` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | other name `CBAPlayerAssigning` | 9 | `CBAPlayerAssigning` |  |
| `CBAForceMovingType` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | - | 9 | `SBAForceMovingType` |  |
| `CBAForcePlayerFoe` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 9 | - |  |
| `CBALeapOntoFoe` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBALeapOntoFoe` | 11 | `SBALeapOntoFoe` |  |
| `CBALocalMove` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBALocalMove` | 12 | `SBALocalMove` |  |
| `CBALoseFoe` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 9 | - |  |
| `CBAMelee` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAMelee` | 11 | `SBAMelee` |  |
| `CBAPlaySound` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 10 | - |  |
| `CBAReuseTactic` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 9 | - |  |
| `CBARunOverFoe` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBARunOverFoe` | 11 | `SBARunOverFoe` |  |
| `CBASetActionSet` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | - | 10 | `SBASetActionSet` |  |
| `CBAShieldStatus` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAShieldStatus` | 9 | `SBAShieldStatus` |  |
| `CBAShoot` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAShoot` | 11 | `SBAShoot` |  |
| `CBAStopMoving` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 9 | - |  |
| `CBAStopTactic` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 9 | - |  |
| `CBAStopUsingTactic` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 9 | - |  |
| `CBAThrowMacroEvent` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAThrowMacroEvent` | 10 | `SBAThrowMacroEvent` |  |
| `CBAThrowPickedTool` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 10 | - |  |
| `CBATryRiding` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | - | 7 | - |  |
| `CBAUnforceMovingType` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 8 | - |  |
| `CBAVTCurrentStateDuration` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 9 | - |  |
| `CBAVTFoeAngle` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 9 | - |  |
| `CBAVTFoeIsPlayer` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 9 | - |  |
| `CBAVTHasFoe` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 9 | - |  |
| `CBAVTHasFoeCandidate` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 9 | - |  |
| `CBAVTHasTool` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 9 | - |  |
| `CBAVTHealth` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | S twin `SBAVTHealth` | 9 | `SBAVTHealth` |  |
| `CBAVTKnockOut` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 9 | - |  |
| `CBAVTMeleeSuccess` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 9 | - |  |
| `CBAVTPlayerDistance` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 9 | - |  |
| `CBAVTWarMeasure` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 9 | - |  |
| `CBAWait` | T01-105 | [`behavior-action-classes`](behavior-action-classes.md) | name | same | 10 | - |  |
| `CBackTemplateHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 9 | - |  |
| `CBackTemplateHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | S twin | S twin `SBackTemplateHudElementParams` | 7 | `SBackTemplateHudElementParams` |  |
| `CBarrierFieldEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | S twin `SBarrierFieldEntity` | 23 | `SBarrierFieldEntity` |  |
| `CBarrierFieldProperties` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | S twin `SBarrierFieldProperties` | 12 | `SBarrierFieldProperties` |  |
| `CBaseEntity` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | name | same | 118 | - |  |
| `CBaseGenericBehaviorEntity` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 10 | `SBaseGenericBehaviorEntity` |  |
| `CBaseGenericBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | - |  |
| `CBaseInventoryItemObsolete` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 7 | - |  |
| `CBaseItemEntity` | T01-108 | [`item-classes`](item-classes.md) | name | S twin `SBaseItemEntity` | 54 | `SBaseItemEntity` |  |
| `CBaseItemParams` | T01-108 | [`item-classes`](item-classes.md) | name | same | 7 | - |  |
| `CBaseItemProperties` | T01-108 | [`item-classes`](item-classes.md) | name | S twin `SBaseItemProperties` | 13 | `SBaseItemProperties` |  |
| `CBaseProjectileEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 45 | `SBaseProjectileEntity` |  |
| `CBaseProjectileParams` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SBaseProjectileParams` | 7 | `SBaseProjectileParams` |  |
| `CBaseProjectileProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 7 | `SBaseProjectileProperties` |  |
| `CBaseProperties` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | name | same | 26 | - |  |
| `CBaseScatterWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SBaseScatterWeaponParams` | 7 | `SBaseScatterWeaponParams` |  |
| `CBaseWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 78 | `SBaseWeaponEntity` |  |
| `CBaseWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SBaseWeaponParams` | 7 | `SBaseWeaponParams` |  |
| `CBaseWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 7 | `SBaseWeaponProperties` |  |
| `CBaseWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SBaseWeaponRenderingParams` | 7 | `SBaseWeaponRenderingParams` |  |
| `CBeamGunInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CBeamGunItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SBeamGunItemEntity` | 8 | - |  |
| `CBeamGunItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SBeamGunItemProperties` | 7 | - |  |
| `CBeamGunWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 26 | `SBeamGunWeaponEntity` |  |
| `CBeamGunWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SBeamGunWeaponParams` | 7 | `SBeamGunWeaponParams` |  |
| `CBeamGunWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CBeamGunWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SBeamGunWeaponRenderingParams` | 7 | - |  |
| `CBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 22 | `SBehaviorEntity` |  |
| `CBehaviorProperties` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 7 | `SBehaviorProperties` |  |
| `CBladesInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CBladesItemEntity` | T01-108 | [`item-classes`](item-classes.md) | name | - | 8 | - |  |
| `CBladesItemProperties` | T01-108 | [`item-classes`](item-classes.md) | rule | - | 7 | - |  |
| `CBodyMovingAnimation` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | same | 7 | - |  |
| `CBombsSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CBombsSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SBombsSimpleHudElementParams` | 7 | - |  |
| `CBossSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 10 | - |  |
| `CBossSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SBossSimpleHudElementParams` | 7 | - |  |
| `CBouncerEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | same | 22 | - |  |
| `CBouncerProperties` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | same | 10 | - |  |
| `CBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 134 | `SBrainEntity` |  |
| `CBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CBuggyBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CBuggyBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CBuggyBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CBuggyPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CBuggyPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CBuggyPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CBulletEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 16 | `SBulletEntity` |  |
| `CBulletParameters` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 4 | - | also in T01-107 |
| `CBulletProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 7 | `SBulletProperties` |  |
| `CBulletTrailEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 10 | `SBulletTrailEntity` |  |
| `CBulletTrailProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 7 | `SBulletTrailProperties` |  |
| `CBulletTrailRenderable` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 13 | - |  |
| `CBulletsInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CBulletsItemEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SBulletsItemEntity` | 9 | - | also in T01-108 |
| `CBulletsItemProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SBulletsItemProperties` | 7 | - |  |
| `CCameraEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | S twin `SCameraEntity` | 51 | `SCameraEntity` |  |
| `CCameraEnvelopeProvider` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | S twin `SCameraEnvelopeProvider` | 17 | `SCameraEnvelopeProvider` |  |
| `CCameraProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | S twin `SCameraProperties` | 12 | `SCameraProperties` |  |
| `CCameras` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | S twin `SCameras` | 6 | - |  |
| `CCannonBallsInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CCannonBallsItemEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SCannonBallsItemEntity` | 9 | - | also in T01-108 |
| `CCannonBallsItemProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SCannonBallsItemProperties` | 7 | - |  |
| `CCannonInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CCannonItemEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SCannonItemEntity` | 8 | - | also in T01-108 |
| `CCannonItemProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SCannonItemProperties` | 7 | - |  |
| `CCannonWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 8 | - |  |
| `CCannonWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SCannonWeaponParams` | 7 | - |  |
| `CCannonWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CCannonWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SCannonWeaponRenderingParams` | 7 | - |  |
| `CChapterInfoEntity` | T01-109 | [`mechanism-classes-world-info`](mechanism-classes-world-info.md) | name | same | 23 | - |  |
| `CChapterInfoProperties` | T01-109 | [`mechanism-classes-world-info`](mechanism-classes-world-info.md) | name | same | 10 | - |  |
| `CChapterListMenuScreen` | T01-112 | [`menu-classes-shell`](menu-classes-shell.md) | name | - | 7 | - |  |
| `CCharacterBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CCharacterBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CCheatsMenuScreen` | T01-112 | [`menu-classes-shell`](menu-classes-shell.md) | name | - | 21 | - |  |
| `CCircularSawBladesWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 7 | - |  |
| `CCircularSawBladesWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SCircularSawBladesWeaponParams` | 7 | - |  |
| `CCircularSawBladesWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CCircularSawInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CCircularSawItemEntity` | T01-108 | [`item-classes`](item-classes.md) | name | - | 8 | - |  |
| `CCircularSawItemProperties` | T01-108 | [`item-classes`](item-classes.md) | rule | - | 7 | - |  |
| `CCircularSawWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 23 | `SCircularSawWeaponEntity` |  |
| `CCircularSawWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SCircularSawWeaponParams` | 7 | `SCircularSawWeaponParams` |  |
| `CCircularSawWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CCircularSawWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SCircularSawWeaponRenderingParams` | 7 | - |  |
| `CCollectChaptersForSortingIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CCollectEnvironmentDamagersIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CCollectMissionObjectivesIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CCollectNavigationalBeaconsIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CCollectSecretsIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CColtItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | - | 8 | - |  |
| `CColtItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | - | 7 | - |  |
| `CColtWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 12 | `SColtWeaponEntity` |  |
| `CColtWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SColtWeaponParams` | 7 | `SColtWeaponParams` |  |
| `CColtWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CColtWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SColtWeaponRenderingParams` | 7 | - |  |
| `CComboWeaponsItemEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 10 | `SComboWeaponsItemEntity` |  |
| `CComboWeaponsItemProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 7 | `SComboWeaponsItemProperties` |  |
| `CComboWeaponsSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CComboWeaponsSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SComboWeaponsSimpleHudElementParams` | 7 | - |  |
| `CContributeToGameTotalStatsIterator` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 8 | `SContributeToGameTotalStatsIterator` |  |
| `CConversationHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 11 | - |  |
| `CConversationHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CCorpseSinkConfig` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | same | 6 | - |  |
| `CCounterSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CCounterSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SCounterSimpleHudElementParams` | 7 | - |  |
| `CCreateMatchMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CCrumbsEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SCrumbsEntity` | 10 | `SCrumbsEntity` |  |
| `CCrumbsProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SCrumbsProperties` | 8 | `SCrumbsProperties` |  |
| `CCurrentStateInfoIterator` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 8 | `SCurrentStateInfoIterator` |  |
| `CDamageHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 10 | `SDamageHudElement` |  |
| `CDamageHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | S twin | S twin `SDamageHudElementParams` | 7 | `SDamageHudElementParams` |  |
| `CDamageMultipliers` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | same | 4 | - | also in T01-113 |
| `CDebrisEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 22 | `SDebrisEntity` |  |
| `CDebrisProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 8 | `SDebrisProperties` |  |
| `CDebugAngle` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugAngle3f` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugBool` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugDumpConfig` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 9 | - |  |
| `CDebugFloat` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugIndex` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugObject` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 19 | - |  |
| `CDebugQV` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugString` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugV3f` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugViewAngleH` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugViewAngleP` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugViewBB` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 9 | - |  |
| `CDebugViewEntityPath` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugViewP` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 10 | - |  |
| `CDebugViewPR` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 8 | - |  |
| `CDebugViewPV` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 9 | - |  |
| `CDebugViewQV` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | - | 9 | - |  |
| `CDetachToolAnimEvent` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 8 | - |  |
| `CDetectorAreaEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | same | 21 | - |  |
| `CDetectorAreaProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 10 | - |  |
| `CDiamondHandEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | same | 31 | - |  |
| `CDiamondHandProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CDifficultyParams` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | same | 5 | - |  |
| `CDisableMacrosIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CDoorAnimEventHandler` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 9 | - |  |
| `CDoorEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | S twin `SDoorEntity` | 39 | `SDoorEntity` |  |
| `CDoorProperties` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | S twin `SDoorProperties` | 13 | `SDoorProperties` |  |
| `CDoubleShotgunInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CDoubleShotgunItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SDoubleShotgunItemEntity` | 8 | - |  |
| `CDoubleShotgunItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SDoubleShotgunItemProperties` | 7 | - |  |
| `CDoubleShotgunWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 11 | `SDoubleShotgunWeaponEntity` |  |
| `CDoubleShotgunWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SDoubleShotgunWeaponParams` | 7 | `SDoubleShotgunWeaponParams` |  |
| `CDoubleShotgunWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CDoubleShotgunWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SDoubleShotgunWeaponRenderingParams` | 7 | - |  |
| `CDrawToolAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | same | 8 | - |  |
| `CDummyFoeBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CDummyFoeBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CDummyFoePuppetEntity` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | name | - | 20 | `SDummyFoePuppetEntity` |  |
| `CDummyFoePuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CDummyFoePuppetProperties` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | S twin | - | 7 | `SDummyFoePuppetProperties` |  |
| `CEffectAnimEventToTerminate` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 7 | `SEffectAnimEventToTerminate` |  |
| `CEnemyPurgedHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 11 | - |  |
| `CEnemyPurgedHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CEntityArray` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 16 | - |  |
| `CEntityHandle` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 4 | - |  |
| `CEntityParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 13 | - |  |
| `CEnvSFXEntity` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | name | same | 11 | - |  |
| `CEnvSFXProperties` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | name | same | 11 | - |  |
| `CEnvironmentDamagerEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | same | 11 | - |  |
| `CEnvironmentDamagerProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 10 | - |  |
| `CExecuteMacrosIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CExitFromRideEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 10 | - |  |
| `CExitFromRideProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | - |  |
| `CExternalJointEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | other name `CTestJointEntity` | 24 | `CTestJointEntity` |  |
| `CExternalJointProperties` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | other name `CTestJointProperties` | 8 | `CTestJointProperties` |  |
| `CExtraLifeHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 11 | - |  |
| `CExtraLifeHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CFeelerSetup` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | - | 3 | - |  |
| `CFindFirstEntityOfTypeIterator` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 8 | `SFindFirstEntityOfTypeIterator` |  |
| `CFindSpawnMarkerIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CFlameEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 12 | - |  |
| `CFlameHitData` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 8 | - |  |
| `CFlameProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 7 | - |  |
| `CFlamerFlameParams` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | same | 8 | - |  |
| `CFlamerItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | - | 8 | - |  |
| `CFlamerItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | - | 7 | - |  |
| `CFlamerProjectileEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 9 | - |  |
| `CFlamerProjectileParams` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | same | 9 | - |  |
| `CFlamerProjectileProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 7 | - |  |
| `CFlamerWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 11 | - |  |
| `CFlamerWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | same | 7 | - |  |
| `CFlamerWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CFlamerWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | same | 7 | - |  |
| `CFlareEntity` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | name | S twin `SFlareEntity` | 16 | `SFlareEntity` |  |
| `CFlareProperties` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | S twin | S twin `SFlareProperties` | 8 | `SFlareProperties` |  |
| `CFlyingKleerBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CFlyingKleerBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CFlyingKleerBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CFlyingKleerPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CFlyingKleerPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CFlyingKleerPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CFoeManagerEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 39 | `SFoeManagerEntity` |  |
| `CFoeManagerProperties` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 7 | `SFoeManagerProperties` |  |
| `CFragsCounterHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 11 | - |  |
| `CFragsCounterHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CFreezingHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 9 | - |  |
| `CFreezingHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CFrozenSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CFrozenSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SFrozenSimpleHudElementParams` | 7 | - |  |
| `CGBAAnimationBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 8 | - |  |
| `CGBAAnimationBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SGBAAnimationBehaviorProperties` |  |
| `CGBALeapOntoFoeBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 12 | `SGBALeapOntoFoeBehaviorEntity` |  |
| `CGBALeapOntoFoeBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SGBALeapOntoFoeBehaviorProperties` |  |
| `CGBALocalMoveBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 8 | - |  |
| `CGBALocalMoveBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SGBALocalMoveBehaviorProperties` |  |
| `CGBAMeleeBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 9 | - |  |
| `CGBAMeleeBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SGBAMeleeBehaviorProperties` |  |
| `CGBARunOverFoeBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 24 | `SGBARunOverFoeBehaviorEntity` |  |
| `CGBARunOverFoeBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SGBARunOverFoeBehaviorProperties` |  |
| `CGBAShootBehaviorEntity` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 9 | - |  |
| `CGBAShootBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SGBAShootBehaviorProperties` |  |
| `CGBAWaitBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 9 | - |  |
| `CGBAWaitBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | - |  |
| `CGBActionListBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 12 | `SGBActionListBehaviorEntity` |  |
| `CGBActionListBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SGBActionListBehaviorProperties` |  |
| `CGameDifficulties` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | same | 4 | - |  |
| `CGameInfoEntity` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 80 | `SGameInfoEntity` |  |
| `CGameInfoProperties` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 7 | - |  |
| `CGameOptionsMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CGameStats` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 8 | - |  |
| `CGenericItemEntity` | T01-108 | [`item-classes`](item-classes.md) | name | same | 8 | - |  |
| `CGenericItemProperties` | T01-108 | [`item-classes`](item-classes.md) | name | same | 7 | - |  |
| `CGenericLeggedBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CGenericLeggedBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CGenericLeggedBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CGenericLeggedPuppetEntity` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | name | - | 9 | `SGenericLeggedPuppetEntity` |  |
| `CGenericLeggedPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SGenericLeggedPuppetParams` | 7 | - |  |
| `CGenericLeggedPuppetProperties` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | name | - | 8 | `SGenericLeggedPuppetProperties` |  |
| `CGenericMotorEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | same | 41 | - |  |
| `CGenericMotorProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CGenericProjectileEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 12 | `SGenericProjectileEntity` |  |
| `CGenericProjectileParams` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SGenericProjectileParams` | 7 | `SGenericProjectileParams` |  |
| `CGenericProjectileProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 7 | `SGenericProjectileProperties` |  |
| `CGlobalGameParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 7 | - |  |
| `CGlobalPlayerParams` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | same | 7 | - | also in T01-113 |
| `CGlobalSpawnerBootTweaks` | T01-109 | [`mechanism-classes-spawner-tweaks`](mechanism-classes-spawner-tweaks.md) | name | same | 7 | - |  |
| `CGlobalSpawnerTweakParams` | T01-109 | [`mechanism-classes-spawner-tweaks`](mechanism-classes-spawner-tweaks.md) | name | same | 7 | - |  |
| `CGlobalSpawningParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 7 | - |  |
| `CGoToPointByPathEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 11 | `GoToPointByPathEntity` |  |
| `CGoToPointByPathProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `GoToPointByPathProperties` |  |
| `CGoToPointEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 10 | `GoToPointEntity` |  |
| `CGoToPointProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `GoToPointProperties` |  |
| `CGrenadeLauncherInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CGrenadeLauncherItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SGrenadeLauncherItemEntity` | 8 | - |  |
| `CGrenadeLauncherItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SGrenadeLauncherItemProperties` | 7 | - |  |
| `CGrenadeLauncherWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 9 | - |  |
| `CGrenadeLauncherWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SGrenadeLauncherWeaponParams` | 7 | - |  |
| `CGrenadeLauncherWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CGrenadeLauncherWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SGrenadeLauncherWeaponRenderingParams` | 7 | - |  |
| `CGrenadesInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CGrenadesItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SGrenadesItemEntity` | 9 | - |  |
| `CGrenadesItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SGrenadesItemProperties` | 7 | - |  |
| `CHandGrenadesInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CHandGrenadesItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SHandGrenadesItemEntity` | 9 | - |  |
| `CHandGrenadesItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SHandGrenadesItemProperties` | 7 | - |  |
| `CHealthItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SHealthItemEntity` | 13 | `SHealthItemEntity` |  |
| `CHealthItemLEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SHealthItemLEntity` | 7 | - |  |
| `CHealthItemLProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SHealthItemLProperties` | 7 | - |  |
| `CHealthItemMEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SHealthItemMEntity` | 7 | - |  |
| `CHealthItemMProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SHealthItemMProperties` | 7 | - |  |
| `CHealthItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SHealthItemProperties` | 7 | - |  |
| `CHealthItemSEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SHealthItemSEntity` | 7 | - |  |
| `CHealthItemSProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SHealthItemSProperties` | 7 | - |  |
| `CHealthItemXLEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SHealthItemXLEntity` | 7 | - |  |
| `CHealthItemXLProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SHealthItemXLProperties` | 7 | - |  |
| `CHealthItemXSEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SHealthItemXSEntity` | 7 | - |  |
| `CHealthItemXSProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SHealthItemXSProperties` | 7 | - |  |
| `CHealthSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CHealthSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SHealthSimpleHudElementParams` | 7 | - |  |
| `CHideAttachmentAnimEvent` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 8 | - |  |
| `CHidePlayerWeaponAttachmentAnimEvent` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 9 | - |  |
| `CHighScoresLevelRankingsMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CHighScoresMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CHighScoresSelectLevelMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CHorizontalWeaponsListHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 19 | `SHorizontalWeaponsListHudElement` |  |
| `CHorizontalWeaponsListHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | S twin | S twin `SHorizontalWeaponsListHudElementParams` | 7 | `SHorizontalWeaponsListHudElementParams` |  |
| `CHovercraftBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CHovercraftBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CHovercraftBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CHovercraftCharacterEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SHovercraftCharacterEntity` | 8 | - |  |
| `CHovercraftCharacterProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | S twin `SHovercraftCharacterProperties` | 7 | - |  |
| `CHovercraftPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SHovercraftPuppetEntity` | 20 | `SHovercraftPuppetEntity` |  |
| `CHovercraftPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SHovercraftPuppetParams` | 7 | `SHovercraftPuppetParams` |  |
| `CHovercraftPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | S twin | S twin `SHovercraftPuppetProperties` | 7 | - |  |
| `CHudElement` | T01-111 | [`hud-classes`](hud-classes.md) | name | - | 14 | `SHudElement` |  |
| `CHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | S twin `SHudElementParams` | 7 | `SHudElementParams` |  |
| `CHudElementsParamsDatabase` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 8 | - |  |
| `CInventoryBase` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 6 | - |  |
| `CInvisibilityHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 7 | - |  |
| `CInvisibilityHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 7 | - |  |
| `CInvisibilityPowerUpItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 7 | - |  |
| `CInvisibilityPowerUpItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | same | 7 | - |  |
| `CInvulnerabilityHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 7 | - |  |
| `CInvulnerabilityHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 7 | - |  |
| `CInvulnerabilityPowerUpItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 7 | - |  |
| `CInvulnerabilityPowerUpItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | same | 7 | - |  |
| `CItemPickedHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 11 | - |  |
| `CItemPickedHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CKickingAbilities` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SKickingAbilities` | 6 | - |  |
| `CKillerEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | S twin `SKillerEntity` | 14 | `SKillerEntity` |  |
| `CKillerProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | S twin `SKillerProperties` | 10 | `SKillerProperties` |  |
| `CKleerBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CKleerBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CKleerBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CKleerPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CKleerPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CKleerPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CKlodovikGunInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CKlodovikGunItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SKlodovikGunItemEntity` | 8 | - |  |
| `CKlodovikGunItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SKlodovikGunItemProperties` | 7 | - |  |
| `CKlodovikGunWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 8 | - |  |
| `CKlodovikGunWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SKlodovikGunWeaponParams` | 7 | - |  |
| `CKlodovikGunWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CKlodovikGunWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SKlodovikGunWeaponRenderingParams` | 7 | - |  |
| `CKlodovikInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CKlodovikItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SKlodovikItemEntity` | 9 | - |  |
| `CKlodovikItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SKlodovikItemProperties` | 7 | - |  |
| `CKnockOutParams` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | S twin `SKnockOutParams` | 6 | - |  |
| `CLPEnvironmentParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | other name `SPuppetEnvironmentParams` | 7 | - |  |
| `CLaunchVelocities` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | same | 5 | - |  |
| `CLauncher` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | S twin `SLauncher` | 11 | - | also in T01-107 |
| `CLeftAmmoHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CLeftAmmoHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | S twin | S twin `SLeftAmmoHudElementParams` | 7 | - |  |
| `CLeggedCharacterEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SLeggedCharacterEntity` | 8 | - |  |
| `CLeggedCharacterProperties` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | S twin `SLeggedCharacterProperties` | 7 | - |  |
| `CLeggedPuppetBodyParams` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | S twin `SLeggedPuppetBodyParams` | 7 | - |  |
| `CLeggedPuppetEntity` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | S twin `SLeggedPuppetEntity` | 128 | `SLeggedPuppetEntity` |  |
| `CLeggedPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SLeggedPuppetParams` | 7 | `SLeggedPuppetParams` |  |
| `CLeggedPuppetProperties` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | S twin `SLeggedPuppetProperties` | 7 | - |  |
| `CLerpToAnimAdjuster` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | - | 13 | - |  |
| `CLifeItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `SLifeItemEntity` | 10 | - |  |
| `CLifeItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `SLifeItemProperties` | 7 | - |  |
| `CLivesSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 9 | - |  |
| `CLivesSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SLivesSimpleHudElementParams` | 7 | `SLivesSimpleHudElementParams` |  |
| `CLocatorEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | S twin `SLocatorEntity` | 17 | `SLocatorEntity` |  |
| `CLocatorProperties` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | same | 10 | - |  |
| `CMCAirAttack` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 10 | - |  |
| `CMCAwayFromFoe` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 9 | - |  |
| `CMCCircleAroundFoe` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 11 | - |  |
| `CMCDiamond` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 9 | - |  |
| `CMCDontMove` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 10 | - |  |
| `CMCDummy` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 9 | - |  |
| `CMCMarkersMovingClosestToFoe` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 18 | - |  |
| `CMCMarkersMovingOrdered` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 17 | - |  |
| `CMCMarkersMovingRandom` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 15 | - |  |
| `CMCMellon` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 9 | - |  |
| `CMCPatternMoving` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 9 | - |  |

## Invariants

1. The 466 rows have 466 distinct class names (checked by the script).
2. Every row's part is the part whose pages cover the class (index, check 3).

## Open questions

None specific to this table: the gaps are listed in [`class-inventory.md`](class-inventory.md), section 8.

## Test vectors

`CAIBrainEntity` is in the first row of this table and belongs to T01-104; `CMCPatternMoving` is in the last row and belongs to T01-105.
