# Class inventory of `Sam2Game.dll`, table 2 of 2: `CMCStayInArea` to `SMoveCharacter` (T01-113)

- **Roadmap task:** T01-113 (last step of the class inventory T01-026; index, checks and gaps: [`class-inventory.md`](class-inventory.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class names, parts, function counts, data names and member table names of the 466 rows (export table, type tables, member tables; the part assignment is the T01-026 split, checked by `tools/re/class_inventory_check.py` against the pages); **medium** for the `rule` rows (the page states a family rule and names the partner class, section 3 of the index).

Prerequisites: [`class-inventory.md`](class-inventory.md) (the columns are explained in its section 3).

## Summary

466 exported classes in name order (ASCII, so `CAI...` precedes `CAdd...`); the other half is [`class-inventory-1.md`](class-inventory-1.md). Every class is in exactly one row of the two tables.

## Evidence

The table was generated from the private export tables of `Sam2Game.dll` (11,610 exports; 932 classes), the type tables of the data (`classes-catalog.md`), the DLL's member tables and the text of the pages of the twelve parts; `tools/re/class_inventory_check.py` re-checks the committed table against the pages and the part counts (it reads only the repository, no game file).

## Layout / rules

| Class | Part | Covering page | Named | In the data | Functions | Member table | Note |
|---|---|---|---|---|---|---|---|
| `CMCStayInArea` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 9 | - |  |
| `CMCTowardsFoe` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | same | 9 | - |  |
| `CMNavigationMovingType` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | - | 17 | - |  |
| `CMSAchievements` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 13 | - |  |
| `CMSAdvancedOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 14 | - |  |
| `CMSAdvancedVideoOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 40 | - |  |
| `CMSAudioOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 20 | - |  |
| `CMSCallAVote` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 13 | - |  |
| `CMSChangeName` | T01-112 | [`menu-classes-shell`](menu-classes-shell.md) | name | - | 11 | - |  |
| `CMSControllerOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 18 | `SMSControllerOptions` |  |
| `CMSCooperativeItemSettings` | T01-112 | [`menu-classes-session`](menu-classes-session.md) | name | - | 17 | - |  |
| `CMSCooperativeSettings` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 26 | - |  |
| `CMSCreateNewProfile` | T01-112 | [`menu-classes-shell`](menu-classes-shell.md) | name | - | 14 | - |  |
| `CMSCustomizeButtons` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 11 | - |  |
| `CMSCustomizeKeys` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 11 | - |  |
| `CMSDeathmatchSettings` | T01-112 | [`menu-classes-session`](menu-classes-session.md) | name | - | 13 | - |  |
| `CMSDebugSettings` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 17 | - |  |
| `CMSDemoEnd` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 12 | - |  |
| `CMSEnterUnlockKey` | T01-112 | [`menu-classes-shell`](menu-classes-shell.md) | name | - | 11 | - |  |
| `CMSExtras` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 10 | - |  |
| `CMSFriendsOptions` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 32 | - |  |
| `CMSGameOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 21 | - |  |
| `CMSGamespy` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 16 | - |  |
| `CMSInGameMenu` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 28 | - |  |
| `CMSItemSettings` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 24 | - |  |
| `CMSLoadGame` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 12 | - |  |
| `CMSLoading` | T01-112 | [`menu-classes-popups`](menu-classes-popups.md) | name | - | 23 | - |  |
| `CMSMainMenuUnderlay` | T01-112 | [`menu-classes-shell`](menu-classes-shell.md) | name | - | 8 | - |  |
| `CMSModSelection` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 13 | - |  |
| `CMSMultiCharDesc` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 11 | - |  |
| `CMSMultiplayer` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 16 | - |  |
| `CMSMultiplayerCharacter` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 14 | - |  |
| `CMSNetworkOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 17 | - |  |
| `CMSOnlineAcceptGameInvite` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 26 | - |  |
| `CMSOnlineFriendList` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 7 | - |  |
| `CMSOnlineLogin` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 25 | - |  |
| `CMSOnlineMessageFeedback` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 10 | - |  |
| `CMSOnlinePlayerList` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 7 | - |  |
| `CMSOnlineSendMessage` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 15 | - |  |
| `CMSOnlineVoicePlayback` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 23 | - |  |
| `CMSOptiMatch` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 9 | - |  |
| `CMSOptiMatchDetails` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 10 | - |  |
| `CMSOptiMatchList` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 8 | - |  |
| `CMSOptionsSelection` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CMSOverlay` | T01-112 | [`menu-classes-shell`](menu-classes-shell.md) | name | - | 8 | - |  |
| `CMSPlayDemo` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 10 | - |  |
| `CMSPlayerComplaint` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 13 | - |  |
| `CMSPlayerFeedback` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 10 | - |  |
| `CMSPlayerList` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 11 | - |  |
| `CMSPlayerOptions` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 14 | - |  |
| `CMSPopupMenuUnderlay` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 8 | - |  |
| `CMSPopupMenuUnderlayLarge` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CMSPopupMenuUnderlayText` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CMSPowerupSettings` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 17 | - |  |
| `CMSProfileSelection` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 15 | - |  |
| `CMSQuickMatch` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 11 | - |  |
| `CMSRecordDemo` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 14 | - |  |
| `CMSRenovationOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 18 | - |  |
| `CMSSaveGame` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 15 | - |  |
| `CMSSelectDifficulty` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 24 | - |  |
| `CMSSelectEpisodeMovies` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 12 | - |  |
| `CMSSelectLevel` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 14 | - |  |
| `CMSSelectMovie` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 12 | - |  |
| `CMSSelectSessionLevel` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 12 | - |  |
| `CMSServerList` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 9 | - |  |
| `CMSSessionSettings` | T01-112 | [`menu-classes-session`](menu-classes-session.md) | name | - | 20 | - |  |
| `CMSSinglePlayer` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 11 | - |  |
| `CMSSinglePlayerSettings` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 24 | - |  |
| `CMSStatistics` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 14 | - |  |
| `CMSTempTranslation` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CMSUnderlay` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 8 | - |  |
| `CMSVideoOptions` | T01-112 | [`menu-classes-options`](menu-classes-options.md) | name | - | 42 | - |  |
| `CMSVotingSettings` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 17 | - |  |
| `CMSWeaponSettings` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 27 | - |  |
| `CMSXboxLive` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 18 | - |  |
| `CMSXboxLiveOptions` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 13 | - |  |
| `CMSXboxLivePassEntry` | T01-112 | [`menu-classes-online`](menu-classes-online.md) | name | - | 13 | - |  |
| `CMSXboxPlayLive` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 8 | - |  |
| `CMainMenu` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 27 | - |  |
| `CManifestedSpawnerBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CManifestedSpawnerBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CManifestedSpawnerBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CManifestedSpawnerPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CManifestedSpawnerPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CManifestedSpawnerPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CMarkersLauncher` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SMarkersLauncher` | 11 | `SMarkersLauncher` |  |
| `CMatchListDetailMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CMaterialSoundRanges` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 4 | - |  |
| `CMeleeDamageAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | S twin `SMeleeDamageAnimEvent` | 9 | `SMeleeDamageAnimEvent` |  |
| `CMilitaryCarBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CMilitaryCarBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CMilitaryCarBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CMilitaryCarPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CMilitaryCarPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CMilitaryCarPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CMiniGunInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CMiniGunItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SMiniGunItemEntity` | 8 | - |  |
| `CMiniGunItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SMiniGunItemProperties` | 7 | - |  |
| `CMiniGunWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 21 | `SMiniGunWeaponEntity` |  |
| `CMiniGunWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SMiniGunWeaponParams` | 7 | `SMiniGunWeaponParams` |  |
| `CMiniGunWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CMiniGunWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SMiniGunWeaponRenderingParams` | 7 | - |  |
| `CMiscPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | other name `CMiscPupeptParams` | 6 | - |  |
| `CMissionObjectiveData` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | - | 3 | - |  |
| `CMissionObjectiveEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | S twin `SMissionObjectiveEntity` | 28 | `SMissionObjectiveEntity` |  |
| `CMissionObjectiveProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | S twin `SMissionObjectiveProperties` | 8 | `SMissionObjectiveProperties` |  |
| `CMountedTool` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | same | 7 | - |  |
| `CMoveCharacterData` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 6 | - |  |
| `CMoviePlayerEntity` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | name | - | 14 | `SMoviePlayerentity` |  |
| `CMoviePlayerProperties` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | S twin | - | 8 | `SMoviePlayerProperties` |  |
| `CMovingAbilities` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SMovingAbilities` | 6 | - |  |
| `CMovingAnimData` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | same | 6 | - |  |
| `CMovingAnimationScheme` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | same | 7 | - |  |
| `CMovingBulletTrail` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | same | 4 | - |  |
| `CMultiSpawnerEntity` | T01-109 | [`mechanism-classes-spawners`](mechanism-classes-spawners.md) | name | S twin `SMultiSpawnerEntity` | 12 | `SMultiSpawnerEntity` |  |
| `CMultiSpawnerProperties` | T01-109 | [`mechanism-classes-spawners`](mechanism-classes-spawners.md) | name | S twin `SMultiSpawnerProperties` | 13 | `SMultiSpawnerProperties` |  |
| `CNPCBomberBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CNPCBomberPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CNPCBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CNPCBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CNPCBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CNPCCivilianBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CNPCCivilianPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CNPCKamikazeBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CNPCKamikazePuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CNPCPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CNPCPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CNPCPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CNPCShooterBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CNPCShooterPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CNapalmItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | - | 9 | - |  |
| `CNapalmItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | - | 7 | - |  |
| `CNavMeshMarkerEntity` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | same | 13 | - |  |
| `CNavMeshMarkerProperties` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | same | 8 | - |  |
| `CNavigationalBeaconData` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | - | 3 | - |  |
| `CNavigationalBeaconEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | S twin `SNavigationalBeaconEntity` | 14 | `SNavigationalBeaconEntity` |  |
| `CNavigationalBeaconProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | S twin `SNavigationalBeaconProperties` | 8 | `SNavigationalBeaconProperties` |  |
| `CNetricsaHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 20 | - |  |
| `CNetricsaHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 7 | - |  |
| `CNetricsaIconHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 13 | - |  |
| `CNetricsaIconHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CObjectActionHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 9 | - |  |
| `CObjectActionHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CObjectManipulation` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 6 | - |  |
| `CObjectManipulationParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | other name `CThrowingParams` | 6 | - |  |
| `COnBootIterator` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 8 | `SOnBootIterator` |  |
| `COnShutDownIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `COnlineNotificationHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 10 | - |  |
| `COnlineNotificationHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | same | 7 | - |  |
| `COperatingParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | S twin `SOperatingParams` | 6 | - |  |
| `CParticleEffectEntity` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | name | other name `CEffectEntity` | 33 | `CEffectEntity` |  |
| `CParticleEffectProperties` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | name | other name `CEffectProperties` | 11 | `CEffectProperties` |  |
| `CParticleEffectSound` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | name | same | 6 | - |  |
| `CPathMarkerEntity` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | S twin `SPathMarkerEntity` | 18 | `SPathMarkerEntity` |  |
| `CPathMarkerProperties` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | S twin | S twin `SPathMarkerProperties` | 8 | `SPathMarkerProperties` |  |
| `CPlasmaInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CPlasmaItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SPlasmaItemEntity` | 9 | - |  |
| `CPlasmaItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SPlasmaItemProperties` | 7 | - |  |
| `CPlasmaRifleInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CPlasmaRifleItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SPlasmaRifleItemEntity` | 8 | - |  |
| `CPlasmaRifleItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SPlasmaRifleItemProperties` | 7 | - |  |
| `CPlasmaRifleWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 21 | - |  |
| `CPlasmaRifleWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SPlasmaRifleWeaponParams` | 7 | `SPlasmaRifleWeaponParams` |  |
| `CPlasmaRifleWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CPlasmaRifleWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SPlasmaRifleWeaponRenderingParams` | 7 | - |  |
| `CPlatformEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | - | 17 | `SPlatformEntity` |  |
| `CPlatformProperties` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | - | 10 | `SPlatformProperties` |  |
| `CPlayerBrainEntity` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 76 | `SPlayerBrainEntity` |  |
| `CPlayerBrainProperties` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 7 | - |  |
| `CPlayerDeathHudElement` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 11 | - | also in T01-111 |
| `CPlayerDeathHudElementParams` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | same | 7 | - | also in T01-111 |
| `CPlayerInventory` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 30 | - |  |
| `CPlayerPuppetCrosshairs` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | same | 7 | - |  |
| `CPlayerPuppetEntity` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 201 | `SPlayerPuppetEntity` |  |
| `CPlayerPuppetParams` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | S twin `SPlayerPuppetParams` | 7 | `SPlayerPuppetParams` |  |
| `CPlayerPuppetProperties` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 7 | - |  |
| `CPlayerSave` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 8 | - |  |
| `CPointsSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 9 | - |  |
| `CPointsSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SPointsSimpleHudElementParams` | 7 | `SPointsSimpleHudElementParams` |  |
| `CPopupMenuAbortGame` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 8 | - |  |
| `CPopupMenuAbortGameLarge` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CPopupMenuCancel` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 12 | - |  |
| `CPopupMenuCancelLarge` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CPopupMenuEnterText` | T01-112 | [`menu-classes-popups`](menu-classes-popups.md) | name | - | 14 | - |  |
| `CPopupMenuEnterTextLarge` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CPopupMenuOk` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 12 | - |  |
| `CPopupMenuOkLarge` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CPopupMenuPleaseWait` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 17 | - |  |
| `CPopupMenuPleaseWaitLarge` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CPopupMenuYesNo` | T01-112 | [`menu-classes-popups`](menu-classes-popups.md) | name | - | 13 | - |  |
| `CPopupMenuYesNoLarge` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CPopupMenuYesNoTimed` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 9 | - |  |
| `CPortalEntity` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | same | 19 | - |  |
| `CPortalProperties` | T01-109 | [`mechanism-classes-movers`](mechanism-classes-movers.md) | name | same | 10 | - |  |
| `CPoseTransitionAnimation` | T01-103 | [`puppet-legged-classes`](puppet-legged-classes.md) | name | same | 7 | - |  |
| `CPossessRideEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 18 | `SPossessRideEntity` |  |
| `CPossessRideProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | `SPossessRideProperties` |  |
| `CPowerUpDurations` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | same | 4 | - |  |
| `CPowerUpHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CPowerUpItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 9 | - |  |
| `CPowerUpItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | same | 7 | - |  |
| `CProfileOptionsMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 10 | - |  |
| `CProfilesMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CPropEntity` | T01-110 | [`worldcontent-models-props`](worldcontent-models-props.md) | name | S twin `SPropEntity` | 61 | `SPropEntity` |  |
| `CPropProperties` | T01-110 | [`worldcontent-models-props`](worldcontent-models-props.md) | name | S twin `SPropProperties` | 19 | `SPropProperties` |  |
| `CPuppetAnimEventHandler` | T01-113 | [`helper-classes`](helper-classes.md) | name | S twin `SPuppetAnimEventHandler` | 12 | - |  |
| `CPuppetAppearAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | - | 8 | - |  |
| `CPuppetBodyParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SPuppetBodyParams` | 7 | - |  |
| `CPuppetDeathParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SPuppetDeathParams` | 6 | - |  |
| `CPuppetDisappearAnimEvent` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 8 | - |  |
| `CPuppetEffectParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SPuppetEffectParams` | 6 | - |  |
| `CPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SPuppetEntity` | 526 | `SPuppetEntity` |  |
| `CPuppetExplodeAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | - | 8 | - |  |
| `CPuppetFireAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | S twin `SPuppetFireAnimEvent` | 9 | `SPuppetFireAnimEvent` |  |
| `CPuppetHealthParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SPuppetHealthParams` | 5 | - |  |
| `CPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SPuppetParams` | 7 | `SPuppetParams` |  |
| `CPuppetProjectileParams` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SPuppetProjectileParams` | 6 | - |  |
| `CPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SPuppetProperties` | 16 | `SPuppetProperties` |  |
| `CPuppetReleaseSpawneeAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | S twin `SPuppetReleaseSpawneeAnimEvent` | 9 | `SPuppetReleaseSpawneeAnimEvent` |  |
| `CPuppetSeatData` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | name | S twin `SPuppetSeatData` | 9 | `SPuppetSeatData` |  |
| `CPuppetSoundAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | S twin `SPuppetSoundAnimEvent` | 7 | `SPuppetSoundAnimEvent` |  |
| `CPuppetSoundRanges` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | same | 4 | - | also in T01-113 |
| `CPuppetsHealthDebugIterator` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 8 | - |  |
| `CRagdollAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | S twin `SRagdollAnimEvent` | 9 | - |  |
| `CRelativeRandomConeLauncher` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | S twin `SRelativeRandomConeLauncher` | 8 | `SRelativeRandomConeLauncher` |  |
| `CRepairItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 11 | - |  |
| `CRepairItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 7 | - |  |
| `CRideAndRiderAnim` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SRideAndRiderAnim` | 7 | `SRideAndRiderAnim` |  |
| `CRideAndSeat` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 6 | - |  |
| `CRideFoeManagerEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 8 | - |  |
| `CRideFoeManagerProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | - |  |
| `CRightAmmoHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CRightAmmoHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | S twin | S twin `SRightAmmoHudElementParams` | 7 | - |  |
| `CRocketLauncherInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CRocketLauncherItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SRocketLauncherItemEntity` | 8 | - |  |
| `CRocketLauncherItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SRocketLauncherItemProperties` | 7 | - |  |
| `CRocketLauncherWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 8 | - |  |
| `CRocketLauncherWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SRocketLauncherWeaponParams` | 7 | - |  |
| `CRocketLauncherWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CRocketLauncherWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SRocketLauncherWeaponRenderingParams` | 7 | - |  |
| `CRocketsInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CRocketsItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SRocketsItemEntity` | 9 | - |  |
| `CRocketsItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SRocketsItemProperties` | 7 | - |  |
| `CRollingBallCharacterEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SRollingBallCharacterEntity` | 8 | - |  |
| `CRollingBallCharacterProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | S twin `SRollingBallCharacterProperties` | 7 | - |  |
| `CRollingBallPuppetEntity` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | name | S twin `SRollingBallPuppetEntity` | 28 | `SRollingBallPuppetEntity` |  |
| `CRollingBallPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SRollingBallPuppetParams` | 7 | `SRollingBallPuppetParams` |  |
| `CRollingBallPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | S twin | S twin `SRollingBallPuppetProperties` | 7 | - |  |
| `CSLMatchListDetailMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CSLMatchListMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CSamNetworkParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 4 | - |  |
| `CSamProjectParams` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | - | 7 | `CSamTestRunParams` |  |
| `CSavingHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 9 | - |  |
| `CSavingHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CScannerHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 10 | `SScannerHudElement` |  |
| `CScannerHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | S twin | S twin `SScannerHudElementParams` | 7 | `SScannerHudElementParams` |  |
| `CSciFiSoldierBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CSciFiSoldierBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CSciFiSoldierBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CSciFiSoldierPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CSciFiSoldierPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CSciFiSoldierPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CSecretEntity` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | same | 22 | - |  |
| `CSecretEntityData` | T01-109 | [`mechanism-classes-triggers`](mechanism-classes-triggers.md) | name | - | 3 | - |  |
| `CSecretProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSeriousBombInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CSeriousBombItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SSeriousBombItemEntity` | 13 | - |  |
| `CSeriousBombItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SSeriousBombItemProperties` | 7 | - |  |
| `CSeriousBombWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 9 | - |  |
| `CSeriousBombWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SSeriousBombWeaponParams` | 7 | - |  |
| `CSeriousBombWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CSeriousBombWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SSeriousBombWeaponRenderingParams` | 7 | - |  |
| `CSeriousDamageHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 7 | - |  |
| `CSeriousDamageHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 7 | - |  |
| `CSeriousDamageItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 8 | - |  |
| `CSeriousDamageItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | same | 7 | - |  |
| `CSeriousJumpHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 7 | - |  |
| `CSeriousJumpHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | other name `CJumpSimpleHudElementParams` | 7 | - |  |
| `CSeriousJumpItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 8 | - |  |
| `CSeriousJumpItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 7 | - |  |
| `CSeriousScoreHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 7 | - |  |
| `CSeriousScoreHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 7 | - |  |
| `CSeriousScoreItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 7 | - |  |
| `CSeriousScoreItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | same | 7 | - |  |
| `CSeriousSpeedHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 7 | - |  |
| `CSeriousSpeedHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 7 | - |  |
| `CSeriousStrengthHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 7 | - |  |
| `CSeriousStrengthHudElementParams` | T01-111 | [`hud-classes`](hud-classes.md) | name | other name `CStrengthSimpleHudElementParams` | 7 | - |  |
| `CSerious_BombInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CSerious_BombItemEntity` | T01-108 | [`item-classes`](item-classes.md) | name | - | 8 | - |  |
| `CSerious_BombItemProperties` | T01-108 | [`item-classes`](item-classes.md) | rule | - | 7 | - |  |
| `CShakerEffectEntity` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | name | S twin `SShakerEffectEntity` | 15 | `SShakerEffectEntity` |  |
| `CShakerEffectProperties` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | rule | S twin `SShakerEffectProperties` | 9 | `SShakerEffectProperties` |  |
| `CShellsInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CShellsItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SShellsItemEntity` | 9 | - |  |
| `CShellsItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SShellsItemProperties` | 7 | - |  |
| `CShieldDescription` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | same | 4 | - |  |
| `CShooterEntity` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | other name `SBulletShooterEntity` | 25 | `SBulletShooterEntity` |  |
| `CShooterProperties` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | - | 7 | - |  |
| `CShowAttachmentAnimEvent` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 8 | - |  |
| `CShowPlayerWeaponAttachmentAnimEvent` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 9 | - |  |
| `CSimpleEffectEntity` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | name | - | 13 | `SSimpleEffectEntity` |  |
| `CSimpleEffectProperties` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | S twin | - | 7 | `SSimpleEffectProperties` |  |
| `CSimpleHudBarParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | same | 7 | - |  |
| `CSimpleHudElement` | T01-111 | [`hud-classes`](hud-classes.md) | name | - | 21 | `SSimpleHudElement` |  |
| `CSimpleHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | name | S twin `SSimpleHudElementParams` | 7 | `SSimpleHudElementParams` |  |
| `CSniperBulletsItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SSniperBulletsItemEntity` | 9 | - |  |
| `CSniperBulletsItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SSniperBulletsItemProperties` | 7 | - |  |
| `CSniperItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SSniperItemEntity` | 8 | - |  |
| `CSniperItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SSniperItemProperties` | 7 | - |  |
| `CSniperWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 22 | `SSniperWeaponEntity` |  |
| `CSniperWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SSniperWeaponParams` | 7 | `SSniperWeaponParams` |  |
| `CSniperWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CSniperWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SSniperWeaponRenderingParams` | 7 | - |  |
| `CSoundAnimEventToTerminate` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 7 | `SSoundAnimEventToTerminate` |  |
| `CSoundEffectEntity` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | name | - | 12 | `SSoundEffectEntity` |  |
| `CSoundEffectProperties` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | S twin | - | 9 | `SSoundEffectProperties` |  |
| `CSpawnDelayerEntity` | T01-109 | [`mechanism-classes-spawners`](mechanism-classes-spawners.md) | name | - | 9 | `SSpawnDelayerEntity` |  |
| `CSpawnDelayerProperties` | T01-109 | [`mechanism-classes-spawners`](mechanism-classes-spawners.md) | name | - | 7 | `SSpawnDelayerProperties` |  |
| `CSpawnEffectCapsuleEntity` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | name | - | 8 | - |  |
| `CSpawnEffectCapsuleProperties` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | rule | - | 7 | - |  |
| `CSpawnEffectEntity` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | name | - | 8 | - |  |
| `CSpawnEffectProperties` | T01-110 | [`worldcontent-effects`](worldcontent-effects.md) | rule | - | 7 | - |  |
| `CSpawnMarkerEntity` | T01-109 | [`mechanism-classes-spawners`](mechanism-classes-spawners.md) | name | same | 12 | - |  |
| `CSpawnMarkerProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerBootTweakCondition` | T01-109 | [`mechanism-classes-spawner-tweaks`](mechanism-classes-spawner-tweaks.md) | name | same | 8 | - |  |
| `CSpawnerBootTweakType` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerBootTweaks` | T01-109 | [`mechanism-classes-spawner-tweaks`](mechanism-classes-spawner-tweaks.md) | name | same | 8 | - |  |
| `CSpawnerEntity` | T01-109 | [`mechanism-classes-spawners`](mechanism-classes-spawners.md) | name | S twin `SSpawnerEntity` | 81 | `SSpawnerEntity` |  |
| `CSpawnerProperties` | T01-109 | [`mechanism-classes-spawners`](mechanism-classes-spawners.md) | name | S twin `SSpawnerProperties` | 8 | `SSpawnerProperties` |  |
| `CSpawnerTweakBarrierCollision` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakChangeSenseRange` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakChangeThreatSensitivity` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakCondition` | T01-109 | [`mechanism-classes-spawner-tweaks`](mechanism-classes-spawner-tweaks.md) | name | same | 7 | - |  |
| `CSpawnerTweakConditionLocation` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakConditionOwnerID` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakConditionOwnerName` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakEnemySize` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | - | 8 | - |  |
| `CSpawnerTweakIgnoreCantMove` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakIgnoreMax` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakIncreaseMulti` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakIncreaseY` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakModifyDelays` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakMustBeShooter` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakNoAutoPatrol` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakNoMulti` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakNoRandom` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakNullTactics` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpawnerTweakParams` | T01-109 | [`mechanism-classes-spawner-tweaks`](mechanism-classes-spawner-tweaks.md) | name | same | 8 | - |  |
| `CSpawnerTweakPatrolSpeed` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakPlacement` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakRandomCone` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakReduce` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 8 | - |  |
| `CSpawnerTweakType` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 7 | - |  |
| `CSpeedPowerUpItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | same | 7 | - |  |
| `CSpeedPowerUpItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | same | 7 | - |  |
| `CSpineAdjuster` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | same | 14 | - |  |
| `CStateCallsInfoIterator` | T01-113 | [`helper-classes-iterators`](helper-classes-iterators.md) | name | - | 8 | `SStateCallsInfoIterator` |  |
| `CStaticBulletTrail` | T01-107 | [`projectile-classes`](projectile-classes.md) | name | same | 4 | - |  |
| `CStaticModelAnimEventHandler` | T01-113 | [`helper-classes`](helper-classes.md) | name | S twin `SStaticModelAnimEventHandler` | 12 | - |  |
| `CStaticModelAnimEventsData` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | S twin `SStaticModelAnimEventsData` | 9 | - |  |
| `CStaticModelEntity` | T01-110 | [`worldcontent-models-props`](worldcontent-models-props.md) | name | same | 81 | - |  |
| `CStaticModelProperties` | T01-110 | [`worldcontent-models-props`](worldcontent-models-props.md) | name | same | 14 | - |  |
| `CStaticSoundEntity` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | name | S twin `SStaticSoundEntity` | 30 | `SStaticSoundEntity` |  |
| `CStaticSoundProperties` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | rule | S twin `SStaticSoundProperties` | 10 | `SStaticSoundProperties` |  |
| `CStaticVibrationEntity` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | name | - | 19 | - |  |
| `CStaticVibrationProperties` | T01-110 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | rule | - | 8 | - |  |
| `CStoreToolAnimEvent` | T01-113 | [`helper-classes-events`](helper-classes-events.md) | name | same | 8 | - |  |
| `CStoredChapterStatus` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 9 | - |  |
| `CStoredLevelStatus` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 6 | - |  |
| `CStrengthItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | - | 7 | - |  |
| `CStrengthItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | - | 7 | - |  |
| `CTacticEntity` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | S twin `STacticEntity` | 13 | `STacticEntity` |  |
| `CTacticProperties` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | S twin `STacticProperties` | 8 | `STacticProperties` |  |
| `CTextToSay` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | - | 6 | - |  |
| `CThinkPointOscilatorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 12 | - |  |
| `CThinkPointOscilatorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | - |  |
| `CThrowDeathToolsAnimEvent` | T01-113 | [`helper-classes`](helper-classes.md) | name | same | 8 | - |  |
| `CThrowDescription` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | same | 7 | - |  |
| `CTimeRemainHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 9 | - |  |
| `CTimeRemainHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | same | 7 | - |  |
| `CTimeSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 8 | - |  |
| `CTimeSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `STimeSimpleHudElementParams` | 7 | - |  |
| `CToolEntityCreator` | T01-113 | [`helper-classes-params`](helper-classes-params.md) | name | same | 7 | - |  |
| `CTreasureItemEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `STreasureItemEntity` | 11 | `STreasureItemEntity` |  |
| `CTreasureItemLEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `STreasureItemLEntity` | 7 | - |  |
| `CTreasureItemLProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `STreasureItemLProperties` | 7 | - |  |
| `CTreasureItemMEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `STreasureItemMEntity` | 7 | - |  |
| `CTreasureItemMProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `STreasureItemMProperties` | 7 | - |  |
| `CTreasureItemProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `STreasureItemProperties` | 7 | - |  |
| `CTreasureItemSEntity` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | name | S twin `STreasureItemSEntity` | 7 | - |  |
| `CTreasureItemSProperties` | T01-108 | [`item-classes-vitals`](item-classes-vitals.md) | rule | S twin `STreasureItemSProperties` | 7 | - |  |
| `CTurretBehaviorEntity` | T01-105 | [`behavior-entity-classes`](behavior-entity-classes.md) | name | - | 8 | - |  |
| `CTurretBehaviorProperties` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | - | 7 | - |  |
| `CTurretBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CTurretBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CTurretBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CTurretCharacterEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `STurretCharacterEntity` | 8 | - |  |
| `CTurretCharacterProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | S twin `STurretCharacterProperties` | 7 | - |  |
| `CTurretPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `STurretPuppetEntity` | 25 | `STurretPuppetEntity` |  |
| `CTurretPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `STurretPuppetParams` | 7 | `STurretPuppetParams` |  |
| `CTurretPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | S twin | S twin `STurretPuppetProperties` | 7 | - |  |
| `CUpdateHighScoresMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CUziInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CUziItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | S twin `SUziItemEntity` | 8 | - |  |
| `CUziItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | S twin `SUziItemProperties` | 7 | - |  |
| `CUziWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 13 | `SUziWeaponEntity` |  |
| `CUziWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SUziWeaponParams` | 7 | `SUziWeaponParams` |  |
| `CUziWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CUziWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SUziWeaponRenderingParams` | 7 | - |  |
| `CVehicleAmmoSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 9 | - |  |
| `CVehicleAmmoSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | same | 7 | - |  |
| `CVehicleSimpleHudElement` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | name | - | 9 | - |  |
| `CVehicleSimpleHudElementParams` | T01-111 | [`hud-classes-simple`](hud-classes-simple.md) | S twin | S twin `SVehicleSimpleHudElementParams` | 7 | - |  |
| `CViewHighScoresMenuScreen` | T01-112 | [`menu-classes`](menu-classes.md) | name | - | 7 | - |  |
| `CVisBoxEntity` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | name | S twin `SVisBoxEntity` | 12 | `SVisBoxEntity` |  |
| `CVisBoxProperties` | T01-110 | [`worldcontent-markers-debug`](worldcontent-markers-debug.md) | S twin | S twin `SVisBoxProperties` | 10 | `SVisBoxProperties` |  |
| `CVotingInfoEntity` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 26 | - |  |
| `CVotingInfoHudElement` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 9 | - | also in T01-111 |
| `CVotingInfoHudElementParams` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | same | 7 | - | also in T01-111 |
| `CVotingInfoProperties` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 7 | - |  |
| `CWayToSeat` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | S twin `SWayToSeat` | 7 | `SWayToSeat` |  |
| `CWeaponAnimEventHandler` | T01-113 | [`helper-classes`](helper-classes.md) | name | - | 11 | - |  |
| `CWeaponIcons` | T01-111 | [`hud-classes`](hud-classes.md) | name | - | 6 | - |  |
| `CWeaponInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | name | - | 7 | - |  |
| `CWeaponItemEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SWeaponItemEntity` | 14 | `SWeaponItemEntity` | also in T01-108 |
| `CWeaponItemProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | S twin `SWeaponItemProperties` | 7 | - |  |
| `CWeaponsListHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 13 | `SWeaponsListHudElement` |  |
| `CWeaponsListHudElementParams` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | S twin | S twin `SWeaponsListHudElementParams` | 7 | `SWeaponsListHudElementParams` |  |
| `CWeaponsMenuHudElement` | T01-111 | [`hud-classes-special`](hud-classes-special.md) | name | - | 15 | `SWeaponsMenuHudElement` |  |
| `CWeaponsMenuHudElementParams` | T01-111 | [`hud-classes-params`](hud-classes-params.md) | S twin | S twin `SWeaponsMenuHudElementParams` | 7 | `SWeaponsMenuHudElementParams` |  |
| `CWereBullBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CWereBullBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CWereBullBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CWereBullPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CWereBullPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CWereBullPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CWheelData` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 6 | - |  |
| `CWheeledVehicleBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CWheeledVehicleBrainParams` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 7 | - |  |
| `CWheeledVehicleBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CWheeledVehicleCharacterEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CWheeledVehicleCharacterProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CWheeledVehiclePuppetEntity` | T01-103 | [`puppet-vehicle-classes`](puppet-vehicle-classes.md) | name | - | 20 | `SWheeledVehiclePuppetEntity` |  |
| `CWheeledVehiclePuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | `SWheeledVehiclePuppetParams` |  |
| `CWheeledVehiclePuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `CWorldInfoEntity` | T01-109 | [`mechanism-classes-world-info`](mechanism-classes-world-info.md) | name | same | 145 | - |  |
| `CWorldInfoProperties` | T01-109 | [`mechanism-classes`](mechanism-classes.md) | name | same | 10 | - |  |
| `CZapGunInventoryItem` | T01-102 | [`players-session-classes`](players-session-classes.md) | rule | - | 7 | - |  |
| `CZapGunItemEntity` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | name | - | 8 | - |  |
| `CZapGunItemProperties` | T01-108 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | rule | - | 7 | - |  |
| `CZapGunWeaponEntity` | T01-106 | [`weapon-classes`](weapon-classes.md) | name | - | 11 | `SZapGunWeaponEntity` |  |
| `CZapGunWeaponParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | S twin | S twin `SZapGunWeaponParams` | 7 | `SZapGunWeaponParams` |  |
| `CZapGunWeaponProperties` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | - | 7 | - |  |
| `CZapGunWeaponRenderingParams` | T01-106 | [`weapon-classes`](weapon-classes.md) | rule | S twin `SZapGunWeaponRenderingParams` | 7 | - |  |
| `CZombieSoldierBrainEntity` | T01-104 | [`brain-classes`](brain-classes.md) | name | - | 8 | - |  |
| `CZombieSoldierBrainProperties` | T01-104 | [`brain-classes`](brain-classes.md) | rule | - | 7 | - |  |
| `CZombieSoldierPuppetEntity` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 8 | - |  |
| `CZombieSoldierPuppetParams` | T01-103 | [`puppet-classes`](puppet-classes.md) | name | - | 7 | - |  |
| `CZombieSoldierPuppetProperties` | T01-103 | [`puppet-classes`](puppet-classes.md) | rule | - | 7 | - |  |
| `SBAList` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 10 | - |  |
| `SBaseEntity` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | name | same | 18 | - |  |
| `SBaseProperties` | T01-110 | [`worldcontent-classes`](worldcontent-classes.md) | name | same | 7 | - |  |
| `SBehaviorAction` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 7 | - |  |
| `SDynamicMoveData` | T01-105 | [`behavior-mover-classes`](behavior-mover-classes.md) | name | - | 6 | - |  |
| `SMoveCharacter` | T01-105 | [`behavior-classes`](behavior-classes.md) | name | same | 19 | - |  |

## Invariants

1. The 466 rows have 466 distinct class names (checked by the script).
2. Every row's part is the part whose pages cover the class (index, check 3).

## Open questions

None specific to this table: the gaps are listed in [`class-inventory.md`](class-inventory.md), section 8.

## Test vectors

`CMCStayInArea` is in the first row of this table and belongs to T01-105; `SMoveCharacter` is in the last row and belongs to T01-105.
