"""Evidence for docs/re/menu-classes.md and its companion pages (T01-112): the menu classes of Sam2Game.dll against the menu layouts in the data.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries). It contains, as constants, names that
the binary itself exports or declares (the 104 exported menu classes with their method names; the member names of the gadget layout classes)
and a few numbers read from the binary once (marked below); everything else is computed from the 99 `Content/SeriousSam2/Menus/*.rsc` layouts.

It prints
  1. the counts of the exported classes and functions (104 / 1,412 / 684 named);
  2. which layout belongs to which exported class (by the layout's file stem, which is what the game opens screens by), which screen class
     identifier each layout carries (the class itself, its S twin or another name), the layouts nobody names and the classes without a layout;
  3. for every handler method `<gadget>_On<Event>` whether `<gadget>` is a gadget of the class's own layout, and which gadget kinds each event belongs to;
  4. the binary's declared member names of the gadget layout classes against the data's type tables (counts, order, types);
  5. the option lists, integer boxes and slider boxes of the layouts, compared with the tables the binary holds (enemy multiplier percentages,
     difficulty values, video quality levels, connection presets, display modes);
  6. the popup layouts.

Usage: python -I tools/re/menu_classes_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import collections, os, re, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, _natural, merged_files, parse  # noqa: E402

# The 104 menu classes exported by Sam2Game.dll (T01-026 split, part T01-112): class -> (exported functions, the named methods besides the
# 7 standard symbols). Names are quoted from the export table.
EXPORTS = {
    "CAssignNewProfileMenuScreen": (7, ""),
    "CChapterListMenuScreen": (7, ""),
    "CCheatsMenuScreen": (21,
        "ExitCheatsMenu PostCreate Step btnAddExtraLife_OnClick btnGiveAllItems_OnClick btnGodMode_OnClick "
        "btnKillAllEnemies_OnClick btnRefreshArmor_OnClick btnRefreshHealth_OnClick btnToggleFly_OnClick "
        "btnToggleGhost_OnClick btnToggleHud_OnClick btnTurboMode_OnClick btnUnlockLevels_OnClick"),
    "CCreateMatchMenuScreen": (7, ""),
    "CGameOptionsMenuScreen": (7, ""),
    "CHighScoresLevelRankingsMenuScreen": (7, ""),
    "CHighScoresMenuScreen": (7, ""),
    "CHighScoresSelectLevelMenuScreen": (7, ""),
    "CMSAchievements": (13,
        "OnCommandPressed OnSetCurrent PopulateAchievementsList PostCreate ibtnArrowDown_OnClick "
        "ibtnArrowUp_OnClick"),
    "CMSAdvancedOptions": (14,
        "CollectProfileOptions OnCommandPressed PostCreate btnDefaultSettings_OnClick btnDefaultSettings_OnYes "
        "opslCrosshairColor_OnChange sldCrosshairSize_OnChange"),
    "CMSAdvancedVideoOptions": (40,
        "ApplyChanges ExecuteFunc OnCommandPressed OnSetCurrent PostCreate ResetVideoOptions StoreInitValues "
        "SyncCrmFromGadgets SyncCrmToGadgets SyncEfxFromGadgets SyncEfxToGadgets SyncGfxFromGadgets "
        "SyncGfxToGadgets SyncOptionsFromGadgets SyncOptionsToGadgets SyncRenFromGadgets SyncRenToGadgets "
        "SyncShaFromGadgets SyncShaToGadgets SyncTexFromGadgets SyncTexToGadgets btnApply_OnClick btnApply_OnYes "
        "btnDefaultSettings_OnClick btnDefaultSettings_OnYes msgExitScreen_OnNo msgExitScreen_OnYes "
        "opslBloom_OnChange opslCrumbs_OnChange opslDynamicShadows_OnChange sldBrightness_OnChange "
        "sldContrast_OnChange sldGamma_OnChange"),
    "CMSAudioOptions": (20,
        "OnCommandPressed OnSetCurrent PostCreate btnDefaultSettings_OnClick btnDefaultSettings_OnYes "
        "opslEAX_OnChange opslMaxSounds_OnChange opslSoftwareEmulation_OnChange sldEffectsVolume_OnChange "
        "sldMasterVolume_OnChange sldMovieVolume_OnChange sldMusicVolume_OnChange sldVoice_OnChange"),
    "CMSCallAVote": (13,
        "PlayerList_PlayerSelected PostCreate btnKickPlayer_OnClick btnNextMap_OnClick btnRestartMap_OnClick "
        "btnTimeBanPlayer_OnClick"),
    "CMSChangeName": (11, "PostCreate btnInvalidProfileName_OnOk eneEnterNameGadget_OnCancel eneEnterNameGadget_OnClick"),
    "CMSControllerOptions": (18,
        "OnCommandPressed OnSetCurrent PostCreate Render opslAutoAim_OnChange opslAxesControls_OnChange "
        "opslAxesControls_OnLoseFocus opslAxesControls_OnSetFocus opslButtonsControls_OnChange "
        "opslButtonsControls_OnLoseFocus opslButtonsControls_OnSetFocus"),
    "CMSCooperativeItemSettings": (17,
        "GetBOOLValue OnSetCurrent PostCreate SetBOOLValue Step UpdateState opslAmmoStays_OnChange "
        "opslDisableSeriousBombs_OnChange opslHealthArmorStays_OnChange opslWeaponsStay_OnChange"),
    "CMSCooperativeSettings": (26,
        "GetBOOLValue OnSetCurrent OnUnsetCurrent PostCreate SetBOOLValue UpdateState islNumLives_OnChange "
        "opslDifficulty_OnChange opslEnableRocketJumping_OnChange opslEnableSprinting_OnChange "
        "opslEnemyMaxMultiplier_OnChange opslEnemyMultiplierPerPlayer_OnChange opslEnemyMultiplier_OnChange "
        "opslEnemyStrengthPerPlayer_OnChange opslExtraEnemyStrength_OnChange opslFriendlyFire_OnChange "
        "opslRespawnInPlace_OnChange opslUseComboWeapons_OnChange opslUseLives_OnChange"),
    "CMSCreateNewProfile": (14,
        "GetNextAvailableName OnCommandPressed PostCreate btnInvalidProfileName_OnOk eneEnterNameGadget_OnCancel "
        "eneEnterNameGadget_OnClick eneEnterNameGadget_OnEscape"),
    "CMSCustomizeButtons": (11, "OnSetCurrent PostCreate ibtnArrowDown_OnClick ibtnArrowUp_OnClick"),
    "CMSCustomizeKeys": (11, "OnSetCurrent PostCreate ibtnArrowDown_OnClick ibtnArrowUp_OnClick"),
    "CMSDeathmatchSettings": (13, "OnSetCurrent PostCreate Step UpdateState islFragLimit_OnChange islTimeLimit_OnChange"),
    "CMSDebugSettings": (17,
        "AddLabelGadget AddOptionSelectGadget ExitMenuScreen GetBOOLValue GetOpslGadgetIdFromEnumName "
        "OnCommandPressed OnSetCurrent PostCreate SetBOOLValue UpdateState"),
    "CMSDemoEnd": (12, "OnCommandPressed OnSetCurrent PostCreate btnQuit_OnClick imgbBuyNow_OnClick"),
    "CMSEnterUnlockKey": (11, "PostCreate RestartGame_OnOk eneEnterNameGadget_OnCancel eneEnterNameGadget_OnClick"),
    "CMSExtras": (10, "OnSetCurrent btnCredits_OnClick btnSpecialThanks_OnClick"),
    "CMSFriendsOptions": (32,
        "ConnectingToHost_OnFailed ConnectingToHost_OnSuccess JoinFriend PostCreate SetMessageAsCurrent Step "
        "UpdateState btnAcceptFriendRequest_OnClick btnAcceptGameInvitation_OnClick "
        "btnBlockFriendRequests_OnClick btnBlockFriendRequests_OnNo btnBlockFriendRequests_OnYes "
        "btnCancelGameInvitation_OnClick btnCancelGameInvitation_OnNo btnCancelGameInvitation_OnYes "
        "btnDeclineFriendRequest_OnClick btnDeclineGameInvitation_OnClick btnJoinFriend_OnCancel "
        "btnJoinFriend_OnClick btnMessageFeedback_OnClick btnPlayVoiceAttachment_OnClick btnRemoveFriend_OnClick "
        "btnRemoveFriend_OnNo btnRemoveFriend_OnYes btnSendGameInvitation_OnClick"),
    "CMSGameOptions": (21,
        "OnSetCurrent SyncOptionsFromGadgets SyncOptionsToGadgets btnDefaultSettings_OnClick "
        "btnDefaultSettings_OnYes opslEnableSprinting_OnChange opslNetricsaSubtitles_OnChange "
        "opslNetricsaVoice_OnChange opslNpcSubtitles_OnChange opslNpcVoice_OnChange opslSamSubtitles_OnChange "
        "opslSamVoice_OnChange opslShowPuppetRadar_OnChange opslUseComboWeapons_OnChange"),
    "CMSGamespy": (16,
        "OnCommandPressed OnSetCurrent SpecifyServer_OnAccept SyncOptionsFromGadgets SyncOptionsToGadgets "
        "btnSpecifyServer_OnClick islMaxBPSDown_OnChange islMaxBPSUp_OnChange opslConnectionType_OnChange"),
    "CMSInGameMenu": (28,
        "GetPlayerLives IsPlayerAlive OnCommandPressed OnCommandReleased OnSetCurrent OnUnsetCurrent PostCreate "
        "Step StopGame_OnFailed StopGame_OnSuccess UpdateState btnContinuePlaying_OnClick "
        "btnContinuePlaying_OnYes btnQuit_OnClick btnQuit_OnYes btnRestartChapter_OnClick btnRestartChapter_OnYes "
        "btnRestartLevel_OnClick btnRestartLevel_OnYes btnResume_OnClick btnStopRecording_OnClick"),
    "CMSItemSettings": (24,
        "GetBOOLValue OnSetCurrent PostCreate SetBOOLValue Step UpdateState opslArmorL_OnChange "
        "opslArmorM_OnChange opslArmorS_OnChange opslArmorXL_OnChange opslArmorXS_OnChange opslArmorXXL_OnChange "
        "opslHealthL_OnChange opslHealthM_OnChange opslHealthS_OnChange opslHealthXL_OnChange "
        "opslHealthXS_OnChange"),
    "CMSLoadGame": (12, "LoadLevelItem_OnSelect OnSetCurrent OutputValue Render Step"),
    "CMSLoading": (23,
        "ConnectingToHost_OnFailed ConnectingToHost_OnSuccess GetLevelData IsInteractive "
        "LoadLevel_FinishedLoading OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate "
        "SessionNotAvailable_OnOk SetImage SetLevel SetupLoadingScreen Step ibtnArrowDown_OnClick "
        "ibtnArrowUp_OnClick"),
    "CMSMainMenuUnderlay": (8, "PostCreate"),
    "CMSModSelection": (13, "OnSetCurrent PostCreate btnChangeMod_OnYes ibtnArrowDown_OnClick ibtnArrowUp_OnClick sclModItem_OnClick"),
    "CMSMultiCharDesc": (11, "OnSetCurrent Render ibtnArrowDown_OnClick ibtnArrowUp_OnClick"),
    "CMSMultiplayer": (16,
        "GamespyConnect_OnFailed GamespyConnect_OnSuccess InstallUpdate_OnNo InstallUpdate_OnYes PostCreate "
        "UpdateCheck_OnFailed UpdateCheck_OnSuccess btnInternet_OnClick btnLocalNetwork_OnClick"),
    "CMSMultiplayerCharacter": (14,
        "MultiCharItem_OnSelect OnCommandPressed OnSetCurrent PostCreate ibtnArrowDown_OnClick "
        "ibtnArrowUp_OnClick sclScrollList_OnTargetHit"),
    "CMSNetworkOptions": (17,
        "OnCommandPressed OnSetCurrent SyncOptionsFromGadgets SyncOptionsToGadgets btnDefaultSettings_OnClick "
        "btnDefaultSettings_OnNo btnDefaultSettings_OnYes islMaxBPSDown_OnChange islMaxBPSUp_OnChange "
        "opslConnectionType_OnChange"),
    "CMSOnlineAcceptGameInvite": (26,
        "AccountLogin_OnFailed AccountLogin_OnSuccess AccountManagement_OnNo AccountManagement_OnYes "
        "AutoUpdate_OnNo AutoUpdate_OnYes ConnectingToHost_OnFailed ConnectingToHost_OnSuccess "
        "GetPassword_OnAccept GetPassword_OnCancel HandleRequiredActions JoinGame MessagePending_OnNo "
        "MessagePending_OnYes NetworkProblems_OnNo NetworkProblems_OnYes PostCreate btnNo_OnClick btnYes_OnClick"),
    "CMSOnlineFriendList": (7, ""),
    "CMSOnlineLogin": (25,
        "AccountLogin_OnFailed AccountLogin_OnSuccess AccountManagement_OnNo AccountManagement_OnYes "
        "AutoUpdate_OnNo AutoUpdate_OnYes CreateNewAccount_OnNo CreateNewAccount_OnYes GetPassword_OnAccept "
        "GetPassword_OnCancel HandleRequiredActions MessagePending_OnNo MessagePending_OnYes NetworkProblems_OnNo "
        "NetworkProblems_OnYes OnCommandPressed PostCreate Step"),
    "CMSOnlineMessageFeedback": (10, "PostCreate btnFeedbackOffensiveMessage_OnClick btnFeedbackSpamMessage_OnClick"),
    "CMSOnlinePlayerList": (7, ""),
    "CMSOnlineSendMessage": (15,
        "FriendListFull_OnOk PostCreate Step UpdateState btnCreateVoiceAttachment_OnClick "
        "btnDeleteVoiceAttachment_OnClick btnPlayVoiceAttachment_OnClick btnSendMessage_OnClick"),
    "CMSOnlineVoicePlayback": (23,
        "IsIdle IsInPlayback IsInRecordingMode OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate "
        "StartPlayback StartRecording Step StepTimed Stop UpdateState btnPlay_OnClick btnRecord_OnClick "
        "btnStop_OnClick"),
    "CMSOptiMatch": (9, "GetBOOLValueFromGadget OnCommandPressed"),
    "CMSOptiMatchDetails": (10, "ConnectingToHost_OnFailed ConnectingToHost_OnSuccess OnCommandPressed"),
    "CMSOptiMatchList": (8, "OnCommandPressed"),
    "CMSOptionsSelection": (7, ""),
    "CMSOverlay": (8, "PostCreate"),
    "CMSPlayDemo": (10, "OnSetCurrent PlayDemoItem_OnSelect Step"),
    "CMSPlayerComplaint": (13,
        "PostCreate btnFeedbackBadName_OnClick btnFeedbackCheating_OnClick btnFeedbackCursing_OnClick "
        "btnFeedbackHarassment_OnClick btnFeedbackScreaming_OnClick"),
    "CMSPlayerFeedback": (10, "PostCreate btnFeedbackGoodAttitude_OnClick btnFeedbackGoodSession_OnClick"),
    "CMSPlayerList": (11, "KickPlayer_OnNo KickPlayer_OnYes OnSetCurrent PlayerList_OnSelect"),
    "CMSPlayerOptions": (14,
        "PostCreate Step UpdateState btnCancelFriendRequest_OnClick btnMutePlayer_OnClick "
        "btnSendFriendRequest_OnClick btnUnmutePlayer_OnClick"),
    "CMSPopupMenuUnderlay": (8, "PostCreate"),
    "CMSPopupMenuUnderlayLarge": (7, ""),
    "CMSPopupMenuUnderlayText": (7, ""),
    "CMSPowerupSettings": (17,
        "GetBOOLValue OnSetCurrent PostCreate SetBOOLValue Step UpdateState opslInvisibility_OnChange "
        "opslInvulnerability_OnChange opslSeriousDamage_OnChange opslSeriousSpeed_OnChange"),
    "CMSProfileSelection": (15,
        "OnCommandPressed OnSetCurrent OpenNewProfileScreen btnDeletedProfile_OnYes btnFreeMoreSpace_OnNo "
        "ibtnArrowDown_OnClick ibtnArrowUp_OnClick sclProfileItem_OnClick"),
    "CMSQuickMatch": (11, "ConnectingToHost_OnFailed ConnectingToHost_OnSuccess OnCommandPressed PostCreate"),
    "CMSRecordDemo": (14,
        "NewRecordDemo_OnAccept OnSetCurrent Overwrite_OnAccept Overwrite_OnYes RecordDemoItem_OnNewRecordDemo "
        "RecordDemoItem_OnSelect Step"),
    "CMSRenovationOptions": (18,
        "OnSetCurrent SyncOptionsFromGadgets SyncOptionsToGadgets btnDefaultSettings_OnClick "
        "btnDefaultSettings_OnYes opslEnableDebris_OnChange opslEnableRagdollDeaths_OnChange "
        "opslEnableUziReloading_OnChange opslEnableUziScreens_OnChange opslLODBias_OnChange "
        "opslMaxDebris_OnChange"),
    "CMSSaveGame": (15,
        "CreateSaveGameFileName NewSaveGame_OnAccept OnSetCurrent Overwrite_OnAccept Overwrite_OnYes "
        "SaveLevelItem_OnNewSaveGame SaveLevelItem_OnSelect Step"),
    "CMSSelectDifficulty": (24,
        "LoadLevel_OnFailed LoadLevel_OnSuccess OnCommandPressed OnSetCurrent PostCreate Proceed "
        "StoreSelectedDifficulty btnEasy_OnClick btnEasy_OnSetFocus btnHard_OnClick btnHard_OnSetFocus "
        "btnNormal_OnClick btnNormal_OnSetFocus btnSerious_OnClick btnSerious_OnSetFocus btnTourist_OnClick "
        "btnTourist_OnSetFocus"),
    "CMSSelectEpisodeMovies": (12, "OnSetCurrent PostCreate SelectEpisodeItem_OnSelect ibtnArrowDown_OnClick ibtnArrowUp_OnClick"),
    "CMSSelectLevel": (14,
        "LoadLevel_OnFailed LoadLevel_OnSuccess OnSetCurrent PostCreate SelectLevelItem_OnSelect "
        "ibtnArrowDown_OnClick ibtnArrowUp_OnClick"),
    "CMSSelectMovie": (12, "OnSetCurrent PostCreate SelectMovieItem_OnSelect ibtnArrowDown_OnClick ibtnArrowUp_OnClick"),
    "CMSSelectSessionLevel": (12, "OnSetCurrent PostCreate SelectLevelItem_OnSelect ibtnArrowDown_OnClick ibtnArrowUp_OnClick"),
    "CMSServerList": (9, "OnCommandPressed SpecifyServer_OnAccept"),
    "CMSSessionSettings": (20,
        "GetBOOLValue OnCommandPressed OnSetCurrent PostCreate SessionName_OnAccept SetBOOLValue UpdateState "
        "islMaxPlayersPC_OnChange islMaxPlayersXbox_OnChange islPrivateSlots_OnChange "
        "opslClientsMayPause_OnChange opslGameType_OnChange opslInfiniteAmmo_OnChange"),
    "CMSSinglePlayer": (11, "OnSetCurrent btnContinuePlaying_OnClick btnCustomLevel_OnClick btnNewCampaign_OnClick"),
    "CMSSinglePlayerSettings": (24,
        "CollectProfileOptions GetBOOLValue OnCommandPressed OnSetCurrent PostCreate SetBOOLValue "
        "SetDifficultyTexture UpdateState btnSelectLevel_OnClick btnStartGame_OnClick opslDifficulty_OnChange "
        "opslEnableRocketJumping_OnChange opslEnableSprinting_OnChange opslEnemyMaxMultiplier_OnChange "
        "opslEnemyMultiplier_OnChange opslUseComboWeapons_OnChange opslUseLives_OnChange"),
    "CMSStatistics": (14,
        "DisplayStaticStats InitDynamicCounters InitSoundChannels OnCommandPressed OnSetCurrent OnUnsetCurrent "
        "Step"),
    "CMSTempTranslation": (7, ""),
    "CMSUnderlay": (8, "PostCreate"),
    "CMSVideoOptions": (42,
        "ApplyChanges ExecuteFunc FillResolutionsGadget GetAvailableResolutions GetResolutionFromGadget "
        "GetResolutionIndex OnCommandPressed OnSetCurrent PostCreate ResetVideoOptions "
        "RestoreInitDisplayModeValues SetResolutionToGadget Step StoreInitValues SyncCvarsFromQuality "
        "SyncCvarsToQuality SyncOptionsFromGadgets SyncOptionsToGadgets btnAdvancedVideoOptions_OnClick "
        "btnAdvancedVideoOptions_OnNo btnAdvancedVideoOptions_OnYes btnApply_OnClick btnApply_OnNo btnApply_OnYes "
        "btnAutoDetect_OnClick btnDefaultSettings_OnClick btnDefaultSettings_OnYes msgExitScreen_OnNo "
        "msgExitScreen_OnYes msgKeepSettings_OnNo msgKeepSettings_OnYes opslDisplayMode_OnChange "
        "opslQuality_OnChange opslTextureSize_OnChange sldGamma_OnChange"),
    "CMSVotingSettings": (17,
        "GetBOOLValue OnSetCurrent PostCreate SetBOOLValue Step UpdateState opslNextMapAllowed_OnChange "
        "opslRestartMapAllowed_OnChange opslTimeBanAllowed_OnChange opslVoteKickAllowed_OnChange"),
    "CMSWeaponSettings": (27,
        "GetBOOLValue OnSetCurrent PostCreate SetBOOLValue Step UpdateState opslAutoShotgun_OnChange "
        "opslBeamGun_OnChange opslCannon_OnChange opslCircularSaw_OnChange opslDoubleColts_OnChange "
        "opslDoubleShotgun_OnChange opslDoubleUzis_OnChange opslGrenadeLauncher_OnChange opslKlodovik_OnChange "
        "opslMinigun_OnChange opslPlasmaRifle_OnChange opslRocketLauncher_OnChange opslSniper_OnChange "
        "opslZapGun_OnChange"),
    "CMSXboxLive": (18,
        "NetworkProblems_OnNo NetworkProblems_OnYes OnSetCurrent PostCreate Step UpdateState XboxDashboard_OnNo "
        "XboxDashboard_OnYes btnSignIn_OnClick btnSignOut_OnClick btnXboxDashboard_OnClick"),
    "CMSXboxLiveOptions": (13,
        "PostCreate Step UpdateState opslAppearOffline_OnChange opslVoiceThroughSpeakers_OnChange "
        "sldVoiceVolume_OnChange"),
    "CMSXboxLivePassEntry": (13, "OnAnyCommandPressed OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate Step"),
    "CMSXboxPlayLive": (8, "PostCreate"),
    "CMainMenu": (27,
        "CheckForGameInvitations HandleDamagedProfiles HandleDroppedProfiles HandleFaultyGameOptions "
        "OnCommandPressed OnInitialInteraction OnSetCurrent PostCreate Render btnDeletedDamagedProfiles_OnNo "
        "btnDeletedDamagedProfiles_OnYes btnFreeMoreSpace_OnNo btnMultiplayer_OnClick btnOptionsLoadFailed_OnOk "
        "btnQuit_OnClick btnQuit_OnYes btnSystemLink_OnClick btnSystemLink_OnNo btnSystemLink_OnYes "
        "btnXboxLive_OnClick"),
    "CMatchListDetailMenuScreen": (7, ""),
    "CPopupMenuAbortGame": (8, "btnOk_OnClick"),
    "CPopupMenuAbortGameLarge": (7, ""),
    "CPopupMenuCancel": (12, "OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate btnCancel_OnClick"),
    "CPopupMenuCancelLarge": (7, ""),
    "CPopupMenuEnterText": (14,
        "OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate SetEditText etEnterText_OnCancel "
        "etEnterText_OnClick"),
    "CPopupMenuEnterTextLarge": (7, ""),
    "CPopupMenuOk": (12, "OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate btnOk_OnClick"),
    "CPopupMenuOkLarge": (7, ""),
    "CPopupMenuPleaseWait": (17,
        "IsInteractive OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate StartPolledProgress "
        "StartTimedProgress Step StepPolled StepTimed"),
    "CPopupMenuPleaseWaitLarge": (7, ""),
    "CPopupMenuYesNo": (13, "OnCommandPressed OnSetCurrent OnUnsetCurrent PostCreate btnNo_OnClick btnYes_OnClick"),
    "CPopupMenuYesNoLarge": (7, ""),
    "CPopupMenuYesNoTimed": (9, "PostCreate Step"),
    "CProfileOptionsMenuScreen": (10, "OnCommandPressed PostCreate btnMultiPlayerCharacters_OnClick"),
    "CProfilesMenuScreen": (7, ""),
    "CSLMatchListDetailMenuScreen": (7, ""),
    "CSLMatchListMenuScreen": (7, ""),
    "CUpdateHighScoresMenuScreen": (7, ""),
    "CViewHighScoresMenuScreen": (7, ""),
}

BIN = {  # class: (parent, the member names the binary declares, in declaration order); data class of the same name
    "CMenuGadgetButtonBarLayout": ("CMenuGadgetLayout", ('mgl_amcbLeftSideButtons', 'mgl_amcbRightSideButtons', 'mgl_fPadding',)),
    "CMenuGadgetChapterListLayout": ("CMGListLayout", ('mgl_idCaptionFont',)),
    "CMenuGadgetFriendsListLayout": ("CMGListLayout", ('mgl_idCaptionFont', 'mgl_pmglStatusBarLayout',)),
    "CMenuGadgetImageLayout": ("CMenuGadgetLayout", ('mgl_ptexTexture', 'mgl_bFlipX', 'mgl_bFlipY',)),
    "CMenuGadgetLabelLayout": ("CMenuGadgetLayout", ('mgl_bBackground', 'mgl_colBackgroundColor', 'mgl_bOutline', 'mgl_colOutlineColor', 'mgl_bStrikeThrough', 'mgl_fStrikeHeight', 'mgl_colStrikeBegin', 'mgl_colStrikeEnd',)),
    "CMenuGadgetLinkLayout": ("CMGButtonLayout", ('mgl_idLink',)),
    "CMenuGadgetListLayout": ("CMenuGadgetLayout", ('mgl_fArrowSize', 'mgl_fArrowOffset', 'mgl_clciColumnInfos', 'mgl_fRowPadding', 'mgl_fRowHeight', 'mgl_colRowBackground', 'mgl_idIndexFont', 'mgl_bDrawMargins', 'mgl_pmglVerticalSlider', 'mgl_idCaptionFont',)),
    "CMenuGadgetOptiMatchListLayout": ("CMGListLayout", ('mgl_idCaptionFont', 'mgl_bShowPing',)),
    "CMenuGadgetProgressBarLayout": ("CMenuGadgetLayout", ('mgl_colBorder',)),
    "CMenuGadgetRenderModelLayout": ("CMenuGadgetLayout", ('mgl_strModelPath', 'mgl_moOrientation', 'mgl_fDistance', 'mgl_bRotation', 'mgl_fRotSpeedH', 'mgl_fFOV', 'mgl_strAnimation', 'mgl_bBackgroundFoil', 'mgl_mfFoil',)),
    "CMenuGadgetScrollListLayout": ("CMenuGadgetLayout", ('mgl_iActiveItem', 'mgl_fScrollSpeed', 'mgl_ctColumns',)),
    "CMenuGadgetSelectBoxLayout": ("CMenuGadgetLayout", ('mgl_vBoxSize', 'mgl_vArrowSize', 'mgl_vArrowOffset', 'mgl_idValueFont', 'mgl_taValueAlignment', 'mgl_fPadding',)),
    "CMenuGadgetIntegerSelectBoxLayout": ("CMGSelectBoxLayout", ('mgl_slMinValue', 'mgl_slMaxValue', 'mg_slDefaultValue', 'mgl_slUpDnIncrement',)),
    "CMenuGadgetOptionSelectBoxLayout": ("CMGSelectBoxLayout", ('mgl_astrOptions', 'mg_iDefaultValue',)),
    "CMenuGadgetSliderBoxLayout": ("CMGSelectBoxLayout", ('mg_ulDefaultValue', 'mg_ulStep', 'mg_ctMarkers', 'mg_fValueMultiplier', 'mg_fValueAdd',)),
    "CMenuGadgetImageSelectBoxLayout": ("CMGSelectBoxLayout", ('mgl_astrOptions', 'mg_iDefaultValue', 'mgl_ptexTexture00', 'mgl_ptexTexture01', 'mgl_ptexTexture02', 'mgl_ptexTexture03', 'mgl_ptexTexture04', 'mgl_ptexTexture05', 'mgl_ptexTexture06', 'mgl_ptexTexture07', 'mgl_ptexTexture08', 'mgl_ptexTexture09', 'mgl_ptexTexture10', 'mgl_ptexTexture11', 'mgl_ptexTexture12', 'mgl_ptexTexture13', 'mgl_ptexTexture14', 'mgl_ptexTexture15', 'mgl_ptexTexture16', 'mgl_ptexTexture17', 'mgl_ptexTexture18', 'mgl_ptexTexture19', 'mgl_iColor', 'mgl_fCrosshairSize',)),
    "CMenuGadgetServerListLayout": ("CMGListLayout", ('mgl_idCaptionFont',)),
    "CMenuGadgetVerticalSliderLayout": ("CMenuGadgetLayout", ('mgl_vArrowSize', 'mgl_vArrowOffset', 'mgl_colBackground',)),
    # the same, from the binary's other member tables (matched to the data class by member count and types):
    "CMenuText": ("-", ("mtxt_strText", "mtxt_idFontTemplate", "mtxt_taTextAlignH", "mtxt_taTextAlignV", "mtxt_vTextOffset")),
    "CMenuGadgetLayout": ("-", ("mgl_idName", "mgl_vPos", "mgl_vSize", "mgl_mtxtGadgetText", "mgl_colColor", "mgl_bHidden", "mgl_plaPlatform")),
    "CMenuControllerButton": ("-", ("mcb_mbtButtonType", "mcb_strText", "mcb_vOffset", "mcb_vSize", "mcb_fStretch")),
    "CListColumnInfo": ("-", ("lci_fWidth", "mgl_taValueAlignment", "mgl_idValueFont", "lci_strCaption", "lci_taHorizontal", "lci_taVertical")),
    "CMGRenderModelOrientation": ("-", ("mgrmo_fHeading", "mgrmo_fPitch", "mgrmo_fBanking")),
    "CMGRenderModelFoil": ("-", ("mgmf_vSize", "mgmf_colFoil")),
    "CMenuScreenLayout": ("CResource", ("msl_idName", "ms_strUnderlay", "ms_strOverlay")),  # three of the four data members: the gadget list has no entry
}


# Which Hungarian prefix of a binary member name fits which data type name (the member tables of the files name the types).
FITS = {
    "b": ("BOOL",), "f": ("FLOAT",), "i": ("INDEX", "SLONG"), "ct": ("INDEX", "SLONG"), "sl": ("SLONG",), "ul": ("ULONG",),
    "id": ("IDENT",), "col": ("COLOR",), "v": ("Vector2f",), "str": ("CString", "CTransString"), "astr": ("CStaticStackArray<CTransString>",),
    "ta": ("ETextAlignHorizontal", "ETextAlignVertical"), "ptex": ("Ptr<CTexture>",), "mtxt": ("CMenuText",), "pla": ("EPlatform",),
    "mbt": ("EMenuButtonTypes",), "mo": ("CMGRenderModelOrientation",), "mf": ("CMGRenderModelFoil",),
    "clci": ("CDynamicContainer<CListColumnInfo>",), "amcb": ("CMenuControllerButton[4]",), "pmgl": ("CMGVerticalSliderLayout*", "CMGStatusBarLayout*"),
}

# Numbers the binary holds that the pages quote; each is compared with the data's option lists below.
ENEMY_MAX_PERCENT = (50, 75, 100, 125, 150, 175, 200, 225, 250, 275, 300, 400, 500, 600, 700, 800, 900, 1000)  # option index -> percent (cooperative screen)
DIFFICULTY_OF_OPTION = (1, 2, 3, 4, 5)  # option index -> game difficulty value (index 2 and any index above 4 give 3)
CONNECTION_PRESETS = ((3000, 12000), (5000, 65536), (7500, 100000))  # option index -> (cli_iMaxOutBPS, cli_iMaxBPS); index 3 = custom
QUALITY_LEVELS = 5  # video quality presets 0..4; the option list has one more entry, CUSTOM
QUALITY_TABLE = {  # cvar -> value per preset level 0..4 (the preset table of the video options screen; 28 variables, in the binary's order)
    "tex_iMaxSize": (10, 11, 12, 13, 14), "lmp_iMaxSize": (10, 11, 12, 13, 14), "tex_iQuality": (3, 3, 0, 0, 0),
    "lmp_iQuality": (3, 0, 0, 0, 0), "gfx_iAntiAliasing": (0, 0, 1, 2, 2), "tex_iFiltering": (21, 22, 22, 22, 22),
    "tex_iAnisotropy": (1, 1, 2, 4, 8), "tex_fLODBias": (0, 0, -3, -3, 0), "ren_fLODBias": (1.5, 1, 0.75, 0.5, 0.25),
    "ren_fOcclusionSizeTreshold": (0.9, 0.7, 0.5, 0.3, 0.1), "ren_iDynamicShadowQuality": (0, 1, 2, 3, 3),
    "ren_fDynamicShadowSlanting": (0.1, 0.25, 0.5, 0.75, 1), "ren_fDynamicShadowCullSize": (0.1, 0.05, 0.03, 0.02, 0.01),
    "ren_fDynamicShadowFadeSize": (0.2, 0.1, 0.05, 0.03, 0.01), "ren_pixDynamicShadowSize": (128, 256, 256, 512, 1024),
    "efx_iLensFlareQuality": (0, 1, 3, 3, 3), "efx_fParticleBias": (0.75, 1, 1.25, 1.5, 2), "efx_fBloomBias": (0, 0.8, 1, 1.1, 1.2),
    "crm_bAllowRendering": (0, 1, 1, 1, 1), "crm_fDistanceBias": (0.5, 0.75, 1, 2, 3), "crm_fDensityBias": (0.5, 0.666, 1, 1.1, 1.25),
    "sha_bFastFlatCubeReflections": (1, 1, 0, 0, 0), "sha_bForceFlatCubeReflections": (1, 0, 0, 0, 0), "sha_bAllowRefractions": (0, 0, 1, 1, 1),
    "sha_iRefractionMip": (3, 2, 1, 0, 0), "sha_bNoOffsetMapping": (1, 1, 0, 0, 0), "d3d_iMaxAllowedPixelProgramVersion": (11, 11, 30, 30, 30),
    "d3d_bForcePixelProgramVersion2": (0, 0, 0, 1, 1)}
# Variables the quality preset sets to a fixed value besides the table (whatever the level): name -> value.
QUALITY_FIXED = {"lmp_bAllowAnisotropy": 0, "mdl_bHardwareSkinning": 1, "mdl_bCacheAnimations": 1, "ren_bOcclusionCulling": 1,
                 "ren_bEffectOcclusion": 0, "ren_bVisibilityGroups": 1, "tex_iBaseMip": 0, "lmp_iBaseMip": 0}
GAME_TYPE_OF_OPTION = (2, 4)  # session settings opslGameType: COOPERATIVE -> 2, DEATHMATCH -> 4 (any other index gives 2)
ENEMY_MULTIPLIER_OF_OPTION = (0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)  # cooperative opslEnemyMultiplier: NONE -> 0, 2X -> 2 ... 11X -> 11
ENEMY_PER_PLAYER_OF_OPTION = (0, 25, 50, 100, 150, 200, 250, 300, 400, 500)  # cooperative opslEnemyMultiplierPerPlayer (percent of an enemy per player)
PERCENT_OF_TEXT = "the cooperative boxes opslExtraEnemyStrength and opslEnemyStrengthPerPlayer: the number before % is stored, NONE gives 0"
POPUP_RESULTS = {"CPopupMenuYesNo": ("OnYes", "OnNo"), "CPopupMenuOk": ("OnOk",), "CPopupMenuCancel": ("OnCancel",),
                 "CPopupMenuEnterText": ("OnAccept", "OnCancel"), "CPopupMenuPleaseWait": ("OnSuccess", "OnFailed"), "CPopupMenuYesNoTimed": ("OnNo",)}

MENU_PARAMETERS = ("mep_slTopBarHeight", "mep_slBottomBarHeight", "mep_slImageTop", "mep_slImageBottom", "men_slOptionsFontHeight",
                   "men_slLabelsFontHeight", "men_bRenderSafeArea")  # declared names of CMenuParameters, in order
GENERIC = 7  # the standard symbols every class exports: two constructors, destructor, two operators, a special function, mdGetDataType


def short(t): return t.replace("CMenuGadget", "").replace("CMG", "").replace("Layout", "")


def tree_files(install):
    """{content tree: {file stem: bytes}} of every `Content/<tree>/Menus/*.rsc` in every .gro archive under the install (patches and
    mods override earlier archives of the same tree: archives are taken in the natural order of their names, `Patch*` last)."""
    gros = []
    for dp, _, fn in os.walk(install):
        for f in fn:
            if f.lower().endswith(".gro"): gros.append((_natural(f), os.path.join(dp, f)))
    out = collections.defaultdict(dict)
    for _, path in sorted(gros):
        try: z = zipfile.ZipFile(path)
        except zipfile.BadZipFile: continue
        with z:
            for i in z.infolist():
                m = re.match(r"(?i)content/([^/]+)/menus/([^/]+)\.rsc$", i.filename)
                if m and not i.is_dir():
                    d = z.read(i)
                    if d[:8] == b"CTSEMETA": out[m.group(1)][m.group(2).lower()] = d
    return out


def load(files):
    """Menu layouts {stem: bytes}: screens (stem, class, underlay, overlay, gadgets with options) and the type tables used."""
    screens = {}; types = {}
    for stem, d in sorted(files.items()):
        n = stem
        r = parse(d); objs, _ = TreeDec(d, r).objects(); idnt = r["idnt"]; T = r["types"]
        for t in T.values():
            if t.members is None: continue
            parent = T[t.parent].name if t.parent in T else None
            types.setdefault(t.name, collections.Counter())[(parent, tuple((m, T[mt].name if mt in T else "?") for m, mt in t.members))] += 1
        rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
        rt = objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]
        if rt.type != "CMenuScreenLayout": continue
        gad = []
        for g in rt.get("CMenuScreenLayout.2"):
            go = objs[g.oid] if isinstance(g, Ref) else g
            if not isinstance(go, Obj): continue
            info = dict(cls=go.type, name=idnt[go.get("CMenuGadgetLayout.1")])
            if go.type == "CMenuGadgetOptionSelectBoxLayout":
                info["opts"] = [o[1][1] for o in go.get("CMenuGadgetOptionSelectBoxLayout.1")]; info["texts"] = info["opts"]
                info["default"] = go.get("CMenuGadgetOptionSelectBoxLayout.2")
            if go.type == "CMenuGadgetIntegerSelectBoxLayout":
                info["range"] = tuple(go.get("CMenuGadgetIntegerSelectBoxLayout.%d" % i) for i in (1, 2, 3, 4))
            if go.type == "CMenuGadgetSliderBoxLayout":
                info["slider"] = tuple(go.get("CMenuGadgetSliderBoxLayout.%d" % i) for i in (1, 2, 3, 4, 5))
            gad.append(info)
        screens[stem] = dict(cls=idnt[rt.get("CMenuScreenLayout.1")], gad=gad,
                                                          under=rt.get("CMenuScreenLayout.3"), over=rt.get("CMenuScreenLayout.4"))
    return screens, types


def handler_parts(h):
    """`btnQuit_OnClick` -> ("btnQuit", "OnClick"); None for names that are not handlers."""
    if "_On" not in h: return None
    a, _, b = h.rpartition("_On")
    return a, "On" + b


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    trees = tree_files(install)
    screens, types = load(trees["SeriousSam2"])
    by_cls = collections.defaultdict(list)
    for stem, s in screens.items(): by_cls[s["cls"]].append(stem)

    nsym = sum(v[0] for v in EXPORTS.values()); meth = sum(len(v[1].split()) for v in EXPORTS.values())
    print(f"1. the exported classes: {len(EXPORTS)}; exported functions {nsym} = {len(EXPORTS) * GENERIC} standard + {meth} named "
          f"(+{nsym - len(EXPORTS) * GENERIC - meth} if negative would mean a mismatch); classes with no named method: "
          f"{sum(1 for v in EXPORTS.values() if not v[1])}; with 1 to 9: {sum(1 for v in EXPORTS.values() if 1 <= len(v[1].split()) <= 9)}; "
          f"with 10 or more: {sum(1 for v in EXPORTS.values() if len(v[1].split()) >= 10)}")
    kinds = collections.Counter()
    for c, (_, ms) in EXPORTS.items():
        for m in ms.split():
            hp = handler_parts(m)
            kinds[hp[1] if hp else m] += 1
    print("   named methods by kind:", dict(kinds.most_common(14)))

    print(f"\n2. layouts: {len(screens)} screens, {sum(len(s['gad']) for s in screens.values())} gadgets; {len(by_cls)} distinct screen classes")
    tier = collections.OrderedDict()
    for c in EXPORTS:
        a = [st for st in screens if st == c.lower()]
        b = [st for st in screens if screens[st]["cls"] == c]
        s_ = [st for st in screens if screens[st]["cls"] == "S" + c[1:]]
        tier[c] = (a, b, s_)
    print(f"   exported class -> layout: file stem equals the class name (case-folded) {sum(1 for t in tier.values() if t[0])}; "
          f"screen class equals the class name {sum(1 for t in tier.values() if t[1])}; screen class is the S twin {sum(1 for t in tier.values() if t[2])}")
    nofile = [c for c, t in tier.items() if not t[0]]
    print("   exported classes with NO layout file of that stem:", nofile)
    stems_matched = {t[0][0] for t in tier.values() if t[0]}
    print("   layout files that no exported class names by stem:", sorted(set(screens) - stems_matched))
    ident = sum(1 for c, t in tier.items() if t[0] and screens[t[0][0]]["cls"] == c)
    print(f"   of the {sum(1 for t in tier.values() if t[0])} stem matches, the screen class is the class itself in {ident}, the S twin in "
          f"{sum(1 for c, t in tier.items() if t[0] and screens[t[0][0]]['cls'] == 'S' + c[1:])}, another name in "
          f"{sum(1 for c, t in tier.items() if t[0] and screens[t[0][0]]['cls'] not in (c, 'S' + c[1:]))}")
    for c, t in tier.items():
        if t[0] and screens[t[0][0]]["cls"] not in (c, "S" + c[1:]): print(f"      {c:36s} layout {t[0][0]:34s} screen class {screens[t[0][0]]['cls']}")
    shared = {k: v for k, v in by_cls.items() if len(v) > 1}
    print("   screen classes with more than one layout file:", {k: sorted(v) for k, v in shared.items()})

    print("\n3. handlers against the layout's gadget names")
    ok = bad = msg = 0; per_suffix = collections.defaultdict(collections.Counter); unmatched = []; msg_names = collections.defaultdict(set)
    for c, (_, ms) in EXPORTS.items():
        st = tier[c][0][0] if tier[c][0] else None
        names = {g["name"]: g["cls"] for g in screens[st]["gad"]} if st else {}
        for m in ms.split():
            hp = handler_parts(m)
            if not hp: continue
            gname, ev = hp
            if gname in names:
                ok += 1; per_suffix[ev][short(names[gname])] += 1
            elif st is None: unmatched.append((c, m, "no layout file"))
            else:
                msg += 1; msg_names[c].add(gname)
    print(f"   handler methods `<name>_On<Event>`: {ok} where <name> is a gadget of the class's own layout; {msg} where it is not (a message name: the caller names a popup, "
          f"the answer comes back as `<message>_OnYes` and so on); {len(unmatched)} in classes without a layout file")
    for ev, cnt in sorted(per_suffix.items()): print(f"      {ev:14s}: gadget kinds {dict(cnt.most_common())}")
    print("   message-style handler names per class:", {c: sorted(v) for c, v in msg_names.items()})
    print("   in classes without a layout file:", unmatched[:12])
    prefixes = collections.Counter(g["name"].rstrip("0123456789")[:4] for s in screens.values() for g in s["gad"])
    pre = collections.Counter()
    for c, (_, ms) in EXPORTS.items():
        for m in ms.split():
            hp = handler_parts(m)
            if hp and hp[0] in {g["name"] for st in tier[c][0] for g in screens[st]["gad"]}: pre[re.match(r"[a-z]+", hp[0]).group(0)] += 1
    print("   gadget-name prefixes of the matched handlers:", dict(pre.most_common()))

    print("\n4. member names the binary declares against the data's type tables (order rule: file order of the data members = declaration order)")
    good = 0
    for cls, (bparent, names) in BIN.items():
        if cls not in types: print(f"   {cls}: not in the data"); continue
        for (dparent, mem), nfiles in types[cls].most_common():
            fit = []; bad = []
            for i, (m, t) in enumerate(mem):
                if i >= len(names): break
                pre_ = re.match(r"^[a-z]+_([a-z]+)", names[i]).group(1)
                pre_ = next((p for p in sorted(FITS, key=len, reverse=True) if pre_.startswith(p)), "?")
                (fit if t in FITS.get(pre_, ()) else bad).append((names[i], t))
            named = all(m == names[i] for i, (m, _) in enumerate(mem) if not m.isdigit() and i < len(names))
            same = len(mem) == len(names)
            good += same and not bad and named
            print(f"   {cls:34s} data {len(mem):2d} / binary {len(names):2d} ({nfiles} files, data base {dparent}, binary base {bparent}): "
                  f"same count {same}; named members at their position {named}; type misfits {bad or 'none'}")
    print(f"   classes with equal counts, no misfit and every data name in place: {good} of {len(BIN)}")
    for cls in ("CMenuGadgetLayout", "CMenuText", "CMenuGadgetSelectBoxLayout", "CMenuGadgetListLayout"):
        (_, mem), _ = types[cls].most_common(1)[0]
        print(f"   {cls}: " + ", ".join(f"{m}={BIN[cls][1][i]}" for i, (m, _) in enumerate(mem)))

    print("\n5. option lists and the numbers read from the binary")
    opts = collections.defaultdict(list)
    for stem, s in screens.items():
        for g in s["gad"]:
            if "opts" in g: opts[g["name"]].append((stem, g["opts"], g["default"]))
    def first(n): return opts[n][0][1] if opts.get(n) else None
    print("   option select boxes:", sum(len(v) for v in opts.values()), "in", len(opts), "distinct names")
    rng = {g["name"]: g["range"] for st in ("cmsnetworkoptions",) for g in screens[st]["gad"] if "range" in g}
    inside = all(rng["islMaxBPSUp"][0] <= o <= rng["islMaxBPSUp"][1] and rng["islMaxBPSDown"][0] <= i_ <= rng["islMaxBPSDown"][1] for o, i_ in CONNECTION_PRESETS)
    print("   the three connection presets lie inside the integer boxes' ranges (up", rng["islMaxBPSUp"][:2], ", down", rng["islMaxBPSDown"][:2], "):", inside,
          "| preset 2 equals the boxes' defaults:", CONNECTION_PRESETS[2] == (rng["islMaxBPSUp"][2], rng["islMaxBPSDown"][2]))
    em = first("opslEnemyMaxMultiplier")
    print("   opslEnemyMaxMultiplier texts equal the binary's percent table:", em == [f"{p}%" for p in ENEMY_MAX_PERCENT], f"({len(em)} options in {len(opts['opslEnemyMaxMultiplier'])} screens)")
    def pct(t): return 0 if t == "NONE" else int(t.rstrip("%"))
    def xs(t): return 0 if t == "NONE" else int(t.rstrip("X"))
    def per_player(t):
        if t == "NONE": return 0
        m = re.match(r"^([\d.]+)X PER (?:(\d+) )?PLAYERS?$", t)
        return round(float(m.group(1)) / int(m.group(2) or 1) * 100)
    xe = first("opslExtraEnemyStrength"); xp = first("opslEnemyStrengthPerPlayer")
    print("   opslExtraEnemyStrength / opslEnemyStrengthPerPlayer texts:", [pct(t) for t in xe], "equal lists:", xe == xp)
    print("   opslEnemyMultiplier: texts as numbers", [xs(t) for t in first("opslEnemyMultiplier")], "equal to the code's mapping:", tuple(xs(t) for t in first("opslEnemyMultiplier")) == ENEMY_MULTIPLIER_OF_OPTION)
    print("   opslEnemyMultiplierPerPlayer texts as percent:", [per_player(t) for t in first("opslEnemyMultiplierPerPlayer")], "equal to the code's table:", tuple(per_player(t) for t in first("opslEnemyMultiplierPerPlayer")) == ENEMY_PER_PLAYER_OF_OPTION)
    sess = [o for st, o, _ in opts["opslGameType"] if st == "cmssessionsettings"][0]
    print("   session opslGameType:", sess, "-> game type values", GAME_TYPE_OF_OPTION, "| the optimatch filter box has an extra ANY:", [o for st, o, _ in opts["opslGameType"] if st == "cmsoptimatch"][0])
    print("   opslDifficulty option list:", first("opslDifficulty"), "-> difficulty values", DIFFICULTY_OF_OPTION, "| in", [s for s, _, _ in opts["opslDifficulty"]])
    print("   opslQuality:", first("opslQuality"), "(", len(first("opslQuality")), "= 5 levels + CUSTOM:", len(first("opslQuality")) == QUALITY_LEVELS + 1, ")")
    print("   opslConnectionType:", first("opslConnectionType"), "in", [s for s, _, _ in opts["opslConnectionType"]], "; presets", CONNECTION_PRESETS)
    print("   opslDisplayMode:", first("opslDisplayMode"))
    print("   opslEnemyMultiplier:", first("opslEnemyMultiplier"))
    print("   quality table:", len(QUALITY_TABLE), "variables x", QUALITY_LEVELS, "levels (all five values present:", all(len(v) == QUALITY_LEVELS for v in QUALITY_TABLE.values()), "); fixed variables:", len(QUALITY_FIXED))
    yn = [(n, v[0][1][:2]) for n, v in sorted(opts.items()) if len(v[0][1]) == 2]
    print(f"   two-entry boxes (the BOOL helpers read index 0 as TRUE): {len(yn)} names; first entries: {dict(collections.Counter(o[0] for _, o in yn))}")
    print("   boxes with 18 entries:", sorted(n for n, v in opts.items() if len(v[0][1]) == 18), "| largest:", max((len(v[0][1]), n) for n, v in opts.items()))
    print("   integer select boxes (min, max, default, step):")
    for stem, s in sorted(screens.items()):
        for g in s["gad"]:
            if "range" in g: print(f"      {stem:26s} {g['name']:24s} {g['range']}")
    print("   slider boxes (default, step, markers, multiplier, add):")
    for stem, s in sorted(screens.items()):
        for g in s["gad"]:
            if "slider" in g: print(f"      {stem:26s} {g['name']:24s} {g['slider']}")

    print("\n6. popup screens in the data (the layouts the popup classes open)")
    for stem, s in sorted(screens.items()):
        if stem.startswith("cpopupmenu") or stem.startswith("cmspopup"):
            print(f"   {stem:28s} class {s['cls']:26s} underlay {s['under']:26s} gadgets {[g['name'] for g in s['gad']]}")
    print("\n7. exported classes without a layout of their stem in the base game")
    print("   ", nofile)
    print("\n8. the other content trees (mods) carry their own Menus folders")
    for tree, files in sorted(trees.items()):
        if tree == "SeriousSam2": continue
        sc, _ = load(files)
        extra = sorted(set(sc) - set(screens)); missing = sorted(set(screens) - set(sc))
        print(f"   {tree}: {len(sc)} layouts; not in the base game: {[(e, sc[e]['cls']) for e in extra]}; base layouts it lacks: {len(missing)}")
        for c in nofile:
            if c.lower() in sc:
                names = {g["name"]: g["cls"] for g in sc[c.lower()]["gad"]}
                hs = [handler_parts(m) for m in EXPORTS[c][1].split() if handler_parts(m)]
                print(f"      {c}: layout of {len(names)} gadgets (class {sc[c.lower()]['cls']}); handlers whose gadget exists: "
                      f"{sum(1 for h in hs if h[0] in names)} of {len(hs)}; unmatched {[h[0] for h in hs if h[0] not in names]}")
    allstems = set().union(*[set(f) for f in trees.values()])
    print("   exported classes with no layout in any tree:", [c for c in EXPORTS if c.lower() not in allstems])
    print("   layouts in some tree that no exported class names by stem:", sorted(allstems - {c.lower() for c in EXPORTS}))

    print("\n9. the menu layout database (Databases/MenuLayout.rsc: the menu parameters and the font table)")
    d = merged_files(install, ".rsc").get("content/serioussam2/databases/menulayout.rsc")
    r = parse(d); objs, _ = TreeDec(d, r).objects(); T = r["types"]; idnt = r["idnt"]
    db = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CMenuLayoutDatabase")
    mp = db.get("CMenuLayoutDatabase.6")
    tt = next(t for t in T.values() if t.name == "CMenuParameters")
    ok = [T[mt].name for _, mt in tt.members] == ["SLONG"] * 6 + ["BOOL"] and len(MENU_PARAMETERS) == 7
    print("   CMenuParameters: data types", [T[mt].name for _, mt in tt.members], "fit the binary's 7 names (slong x6, bool):", ok)
    print("   values:", {n: v for n, (_, v) in zip(MENU_PARAMETERS, mp)})
    print("   database: screens listed", len(db.get("CMenuLayoutDatabase.1")), "| font instances", len(db.get("CMenuLayoutDatabase.2")), "| default font template",
          idnt[db.get("CMenuLayoutDatabase.3")], "| grid sizes", db.get("CMenuLayoutDatabase.4"), "| grid index", db.get("CMenuLayoutDatabase.5"))


if __name__ == "__main__":
    main()

