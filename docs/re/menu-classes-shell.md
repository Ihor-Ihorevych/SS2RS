# Menu classes: main menu, profiles, in-game menu, cheats and the empty screens (T01-112, page 1 of 6)

- **Roadmap task:** T01-112 (overview: [`menu-classes.md`](menu-classes.md); this page covers the 25 classes of the "shell" group)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, the layouts, the gadget names and the handler-to-gadget matches (script); **medium** for rules marked "read" (one decompiled reading); **low** for "(profiled)" and "(name)" purposes and for the Open questions. Nothing was run.

Prerequisites: [`menu-classes.md`](menu-classes.md) (basis marks, the base interface, the popup protocol), `gui.md`, `menus-tables.md`.

## Summary

The shell group is everything around the **start of the game**: the main menu with its start-up checks, the profile screens, the extras, the mod list, the in-game (pause) menu, the cheat screen, the debug settings and the demo end screen; plus the **empty screens** (classes with no method of their own: the high score screens, the chapter list, two unused profile screens) and the three **background screens** (underlay, overlay, main menu underlay). The 25 classes export **289 functions** (175 standard + 114 named); 16 have named methods, 9 export only the standard seven and are pure layouts or unused.

## Evidence

The 25 classes' exported names (`tools/re/menu_classes_verify.py`, sections 1 to 3), the profiles of all their 114 named methods and the decompiled bodies of `CMainMenu` (all start-up checks, quit, multiplayer, render), `CMSInGameMenu` (commands, enable, lives, restart, quit), `CCheatsMenuScreen` (enable table, cheats), `CMSProfileSelection`, `CMSCreateNewProfile` and `CMSModSelection` (partly). Layout facts come from `gui.md`, `menus-tables.md` and the script. **Not checked:** the exact enable condition of each in-game menu gadget; the effect bodies of the cheat functions (they live in `CGameInfoEntity`); the profile storage (`player-rules.md`, `save-game.md`).

## Layout / rules

### 1. `CMainMenu` (layout `cmainmenu`, 14 gadgets; 20 named methods)

**Layout (data).** `lblVersion` (label), `bbrButtonBar`, six links (`lnkSinglePlayer`, `lnkPlayDemos`, `lnkOptions`, `lnkProfileSettings`, `lnkSelectMod`, `lnkExtras`), four buttons (`btnMultiplayer` PC only, `btnSystemLink` and `btnXboxLive` Xbox only, `btnQuit` both), two images. Underlay `CMSMainMenuUnderlay`, overlay `CMSOverlay`. The links need no handler (`menus-tables.md` has their targets).

**Methods.**

| Method | Purpose | Basis |
|---|---|---|
| `PostCreate` | after the gadgets exist, **focuses `lnkSinglePlayer`** (the first entry of the menu) | read |
| `OnSetCurrent` | **graphics adapter check**: if a stored adapter name exists and differs from the name of the adapter now in use, shows an OK popup (`CPopupMenuOkLarge`, message `GraphicCardChanged`) telling the player to run **Auto-Detect Quality** of the video options (`menu-classes-options.md`), then stores the current name | read |
| `OnCommandPressed` | (a) command 21 with a non-zero cheat level opens `CCheatsMenuScreen`; (b) command 12 (back) is swallowed; (c) command 11 or 31 (accept/start) first runs the **initial interaction** (below) once, then **the three start-up checks** in order, and only when all pass hands the command on to the base class (which activates the focused gadget) | read |
| `OnInitialInteraction` | if a default online login is allowed, gives the login the controller that pressed the key and starts it (the automatic Xbox Live sign-in); an exported static flag `ms_bInitialInteraction` makes it run once per start of the game | read |
| `HandleDamagedProfiles` | if the number of **damaged profiles** is above 0, opens a yes/no popup (message `btnDeletedDamagedProfiles`, "%1 profile(s) on hard disk are damaged. Select YES to delete these profiles."); returns 0 (command consumed) when it opened a popup | read |
| `btnDeletedDamagedProfiles_OnYes` / `_OnNo` | deletes the damaged profiles; if any remain opens an OK popup (message `FailedToDeleteProfiles`, "Unable to delete %1 damaged profile(s)."); `_OnNo` does nothing | read / name |
| `HandleDroppedProfiles` | if profiles were **dropped at load** because the maximum count was reached, opens an OK popup (message `Too many profiles`, "Max %1 profile count has been reached. %2 profile(s) did not load successfully.") and clears the counter | read |
| `HandleFaultyGameOptions` | if the load status of the game options is the **failed** value (2), opens a large OK popup (message `btnOptionsLoadFailed`, "Unable to load game settings. Game settings will be reset to defaults. Select OK to continue.") | read |
| `CheckForGameInvitations` | empty (one return) | read |
| `btnFreeMoreSpace_OnNo`, `btnOptionsLoadFailed_OnOk` | empty answers (the popups just close) | read |
| `Render` | looks up the label `lblVersion`, clears a field of it, and **draws the version text itself**: `v<a>.<b>.<c> : <d>` (the four numbers of the engine's version record) in a fixed font at a fixed screen position in the menu's virtual canvas (490, 81), colour 0xFF5F5F5F | read, low for the details |
| `btnQuit_OnClick` / `btnQuit_OnYes` | yes/no popup (`SPopupMenuYesNo`, message `btnQuit`, text "Are you serious?", focus on `btnNo`); on yes closes the screen and calls the engine's quit | read |
| `btnMultiplayer_OnClick` | PC multiplayer entry: if the player is **not online**, starts the online connection and opens a PleaseWait popup (message `GamespyConnect`, "Connecting"); if online, opens `CMSProfileSelection` and sets its **caller gadget** to `btnInternet` (so the profile selection continues to `CMSGamespy`). The same machine code serves `CMSMultiplayer::btnInternet_OnClick` | read |
| `btnSystemLink_OnClick` (empty), `_OnNo` (empty), `_OnYes` | Xbox system link: on yes logs out of Xbox Live and opens `CMSProfileSelection` with caller `btnSystemLink` | read |
| `btnXboxLive_OnClick` | opens `CMSXboxLive` | read |

**The start-up checks (read, medium).** On the **first accept or start command** the main menu does, in this order: the initial interaction (default login), then it stops at the first of: damaged profiles, dropped profiles, faulty game options, and a fourth check whose purpose is not read (a profile function with argument 0). Each check that shows a popup consumes the command, so the player must answer the popup and press again. The code also compares the identifier of the gadget under the focus with a constant before running the checks (the constant is not read: T01-197). The order matters for a port: damaged profiles first, game options last.

### 2. Profile screens (6 classes)

| Class | Layout (gadgets) | Named methods | What they do |
|---|---|---|---|
| `CMSProfileSelection` | `cmsprofileselection` (class `SMSProfileSelection`, 6: caption, a scroll list `sclScrollList`, two arrow image buttons) | 8 | The **list of player profiles**. `OnSetCurrent` fills the list from the stored profiles and may open the new-profile screen (`OpenNewProfileScreen` is called from here and from `OnCommandPressed`; profiled); `OnCommandPressed` (261 bytes, read in part) handles the **delete** command: yes/no popup `ETRSMainMenu.DeleteProfile` "Are you sure you want to delete this player? Game progress and settings will be lost." (message `btnDeletedProfile`); `btnDeletedProfile_OnYes` deletes the profile and returns to the main menu (profiled); `OpenNewProfileScreen` opens `SMSCreateNewProfile` unless the number of profiles has reached the console variable **`plp_iMaxPlayerProfiles`**, in which case it shows a large OK popup (message `Too many profiles`); **`sclProfileItem_OnClick`** selects the profile and then **opens the next screen by the caller gadget** (`menu-classes.md` section 5); arrows scroll the list. |
| `CMSCreateNewProfile` | `cmscreatenewprofile` (class `SMSCreateNewProfile`, 4: a name entry `eneEnterNameGadget`) | 7 | `PostCreate` puts the proposed name in the entry (from `GetNextAvailableName`: it tries nine built-in names, then a name plus a counter `%1 %2`, until one is not used; profiled); `eneEnterNameGadget_OnClick` (1,001 bytes, profiled) **validates** the name: too short, duplicate or failed creation each open an OK popup (message `btnInvalidProfileName`; keys `ETRSProfile.NameTooShort`, `.DuplicatePlayerName`, `.FailedToCreatePlayer`), otherwise creates the profile and **continues like the profile selection** (single player, the single player settings, the level list, the server list, the Xbox play screen, the profile settings, according to the caller gadget); `_OnCancel` and `_OnEscape` return to the main menu or the profile settings. |
| `CMSChangeName` | `cmschangename` (`SMSChangeName`, 4) | 4 | Same name entry for an existing profile: `PostCreate` fills it with the current name; `eneEnterNameGadget_OnClick` validates and renames (too-short name: OK popup `btnInvalidProfileName`); `btnInvalidProfileName_OnOk` refocuses the entry; `_OnCancel` goes back. |
| `CMSEnterUnlockKey` | `cmsenterunlockkey` (`SMSEnterUnlockKey`, 5: two labels, a name entry) | 4 | The **unlock key** entry of the demo: `PostCreate` writes `System ID: ` and the machine's id into `lblTitle`; `eneEnterNameGadget_OnClick` checks the key and shows `ETRSEnterUnlockKey.KeyOK` ("The key appears to be correct. Please restart the game now.") with answer `RestartGame_OnOk`, or `KeyBad` (message `WrongKey`). The key check calls a product-state function (profiled; the callee name in the export list is a shared stub, not used). |
| `CProfileOptionsMenuScreen` | `cprofileoptionsmenuscreen` (`SProfileOptionsMenuScreen`, 8: links to the name change, controls, advanced options and controller screens, one button) | 3 | The **player settings hub**. `PostCreate` focuses `lnkChangeName`; `btnMultiPlayerCharacters_OnClick` opens `CMSMultiplayerCharacter` (`menu-classes-play.md`); `OnCommandPressed` (44 bytes) passes commands on. |
| `CAssignNewProfileMenuScreen`, `CProfilesMenuScreen` | none | 0 | Exported with the seven standard functions only and **no layout** in this install: unused (or opened by a name the data lacks). |

### 3. `CMSInGameMenu` (layout `cmsingamemenu`, class `SInGameMenu`, 17 gadgets; 21 named methods)

The **pause menu** (underlay `CMSPopupMenuUnderlay`, no overlay). Layout (data): `btnContinuePlaying`, `btnRestartChapter` (Xbox only), `btnRestartLevel` (Xbox only), `btnQuit`, `btnStopRecording` (PC only), the links `lnkPlayerSettings`, `lnkOptions`, `lnkXboxLive` (Xbox), `lnkPlayerList` (Xbox), `lnkPlayerListPC`, `lnkSaveGame`, `lnkLoadGame`, `lnkRecordDemo`, `lnkCallAVote` (all PC only), an online notification gadget, an arranger and a button bar. The same class runs a second layout, `cmscallavote` (class `SInGameMenu`).

**Methods.**

| Method | Purpose | Basis |
|---|---|---|
| `OnSetCurrent` | clears the "back pressed" flag and **enables or disables the gadgets** from the game state: single player or not (`netIsLocal`), the local player's **lives** and **alive** state, multiplayer, online, server or client. The gadgets set by name are `btnContinuePlaying`, `btnRestartChapter`, `btnRestartLevel`, `lnkCallAVote`, `lnkLoadGame`, `lnkPlayerList`, `lnkPlayerListPC`, `lnkSaveGame`, `lnkXboxLive`; finally it focuses a gadget by name. Which condition feeds which gadget is **not mapped** (T01-198) | read in part, low for the matrix |
| `UpdateState` / `Step` | `Step` calls `UpdateState` every frame; it shows the **cheat** entry (a button-bar entry with the text `cheats`, the button type 8) when the cheat level is non-zero, and shows `btnStopRecording` and hides `lnkRecordDemo` while a demo is being recorded | read |
| `OnCommandPressed` / `OnCommandReleased` | command 12 (back) on **press** only sets a flag; the menu **closes on the release** of 12 (so the same key press that opened it does not close it again); command 21 opens the cheat screen when the cheat level is non-zero | read |
| `GetPlayerLives`, `IsPlayerAlive` | scan the first **17** player slots of the game info entity for the local player puppet and return its lives or whether it is alive; if there is no game info entity they close the menu | read |
| `btnContinuePlaying_OnClick` / `_OnYes` | allowed only when the local player has **no lives left and is dead** (single player): asks "Are you sure you want to continue playing? Your score will be frozen until the end of this level." and on yes sets the game info flag that **freezes the score** and closes the menu; in every other case just closes the menu | read |
| `btnResume_OnClick` | closes the menu (no gadget named `btnResume` in the layout: leftover) | read |
| `btnRestartChapter_OnClick` / `_OnYes` | yes/no ("All progress in this chapter will be lost."); on yes asks the **world info entity** to restart (one virtual call with the argument 3) | read |
| `btnRestartLevel_OnClick` / `_OnYes` | yes/no ("All progress in this level will be lost."); on yes asks the world info to restart the current level (arguments: the level, mode 5, 3) | read |
| `btnQuit_OnClick` / `_OnYes` | yes/no ("Do you want to leave the game? All progress in this chapter will be lost."; **on the server** the text says that leaving ends the session); on yes, when networked and a server, opens a **PleaseWait** popup "Stopping game" (message `StopGame`) and waits for the network to stop; otherwise stops the game at once and returns to the main menu | read |
| `StopGame_OnSuccess`, `StopGame_OnFailed` | both **stop the session and leave the game** (three calls; the machine code is shared with `CMSLoading::SessionNotAvailable_OnOk` and `CPopupMenuAbortGame::btnOk_OnClick`) | read, low |
| `btnStopRecording_OnClick` | stops demo recording (1 call) | profiled |
| `PostCreate`, `OnUnsetCurrent` | base calls | read |

### 4. `CCheatsMenuScreen` (layout `ccheatsmenuscreen`, class `SCheatsMenuScreen`, 14 gadgets; 14 named methods)

A **cheat panel**. Layout (data): the link `lnkChapters` (to the chapter list `SChapterListMenuScreen`), nine buttons, an arranger. The code has handlers for **11** buttons; the layout holds **9**.

- **Enable by cheat level (read, medium).** `Step` runs every frame and sets, for each of ten gadgets, the enabled flag to "the global cheat level is at least the gadget's level". **Level 1:** `btnRefreshHealth`, `btnAddExtraLife`, `btnGiveAllItems`, `btnGodMode`, `btnUnlockLevels`. **Level 2:** `lnkChapters`, `btnTurboMode`, `btnKillAllEnemies`, `btnToggleHud`, `btnToggleGhost`. The global is non-zero only when cheats are allowed; it is probably `sam_iEnableCheats` (`console-config.md`; `main-loop.md` found a cheat level of exactly 2 for the single-player time-scale key: **consistent**). The nine layout buttons are exactly the nine in `Step`'s table (script: the layout has `btnGiveAllItems`, `btnGodMode`, `btnTurboMode`, `btnKillAllEnemies`, `btnRefreshHealth`, `btnAddExtraLife`, `btnToggleHud`, `btnToggleGhost`, `btnUnlockLevels`).
- **What each handler does (read for the first four; profiled for the others).** The cheat buttons call the **game info entity's cheat functions** and close the panel: `OnAddLivesCheat` (extra life), `OnGiveAllCheat` (all items), `OnGodCheat` (god mode), `OnKillAllCheat` (kill all enemies), `OnArmorCheat` (`btnRefreshArmor`, no button), `OnHealthCheat`, `OnFlyCheat` (`btnToggleFly`, no button), `OnGhostCheat`, `OnTurboCheat`. `btnToggleHud_OnClick` flips a global HUD-visible switch (read). `btnUnlockLevels_OnClick` takes the profile and **marks every level of a list of level names as unlocked**, then closes (read in part). `ExitCheatsMenu` closes the panel; if the gadget with focus is not found it goes back instead.
- The cheat functions of `CGameInfoEntity` are specified with the players and session classes (`players-session-classes.md`); their exact effects: T01-198.

### 5. Extras, achievements, mods (3 classes)

| Class | Layout (gadgets) | Named methods | What they do |
|---|---|---|---|
| `CMSExtras` | `cmsextras` (8: links `lnkAchievements`, `lnkMovies`; buttons `btnSpecialThanks`, `btnCredits`) and the second layout `ccreditsmenuscreen` (7) | 3 (+7 standard) | `OnSetCurrent` focuses `lnkMovies` (profiled); `btnCredits_OnClick` and `btnSpecialThanks_OnClick` each **start a movie** (`Movies/Ntsc/Demo_01.ogv` and `Demo_02.ogv`: the strings in the code; they look like placeholder names, `cutscenes.md` lists the movies of the install) (profiled). |
| `CMSAchievements` | `cmsachievements` (9: three labels, a scroll list `sclScrollList`, a progress bar `pbProgressBar`, two arrow buttons) | 6 | `PostCreate` calls `PopulateAchievementsList` (896 bytes): builds one **list item per achievement** from the achievement table (each item has a name, a description, a thumbnail path and an unlocked flag: the declared members `mg_strAchievementName`, `mg_strAchievementDescription`, `mg_strThumbnailPath`, `mg_bUnlocked` of the list item class) in the font `SeriousVerdana`, and writes `lblAchievmentsProgress` as `%1/%2` (unlocked of total) and fills `pbProgressBar`; the arrow buttons scroll the list (profiled). |
| `CMSModSelection` | `cmsmodselection` (6: scroll list, two arrows) | 6 | `OnSetCurrent` (675 bytes, read): builds the **mod list**: first the three **known mods** `SeriousSam2`, `InSamnity2`, `Sam2Renovation`, then every **`*.mod` file found under `Content`** whose name is none of those three nor `Example` (the console logs `Mod: <name>`); the description of each comes from the **`.nfo` file with the same name** (the code replaces `.mod` by `.nfo`; for the three known ones the files `Content/SeriousSam2.nfo`, `Content/InSamnity2.nfo`, `Content/Sam2Renovation.nfo`). `sclModItem_OnClick` asks yes/no (`ETRSSelectModMenu.ChangeMods` "Are you sure you want to change mods?", message `btnChangeMod`); `btnChangeMod_OnYes` (233 bytes) switches the mod and restarts the game's module loading (`mods.md`; profiled). |

### 6. Debug settings, demo end, translation

- **`CMSDebugSettings`** (layout `cmsdebugsettings`, 77 gadgets: **67 option boxes** that all list `No` first, 5 labels, 2 image buttons, an arranger; 10 named methods). A **debug options screen**: `UpdateState` (1,109 bytes) builds the screen from the debug enumerations (`EDebugClasses`, `DebugDetails`, `DebugDetails2`): it adds a heading label for each group (`--- Debugging Options ---`, `--- Debug Classes Options ---`, `--- Debug Details Options ---`, `--- Debug Details 2 Options ---`) and one **option box per enumeration value** (`AddLabelGadget`, `AddOptionSelectGadget`, named `opsl<EnumName>`; `GetOpslGadgetIdFromEnumName`), plus the master switch `ENABLE DEBUGGING` (`opslDebuggingAllowed`). `ExitMenuScreen` (the only exported override of that slot) writes the boxes back into the debug variables. `OnCommandPressed` scrolls the arranger (arrow buttons). The screen also exports `GetBOOLValue` and `SetBOOLValue`, whose machine code is shared with the other settings screens (index 0 reads as TRUE, `menu-classes-session.md`); in this layout the first entry is `No`, so whether the debug screen uses the pair at all is open (T01-206).
- **`CMSDemoEnd`** (layout `cmsdemoend`, class `CMSDemoEnd`, 4: image, label, button `btnQuit`, image button `imgbBuyNow`; 5 named methods). The **end screen of the demo**: `OnSetCurrent` (528 bytes) reads the files `Content/SeriousSam2/GameFeatures*.txt` into the label `lblGameFeatures` and picks up a `LINK=` line; `imgbBuyNow_OnClick` opens that link; `btnQuit_OnClick` quits. Product state; unused in a retail install (profiled).
- **`CMSTempTranslation`** (no layout; 0 named methods): its constructor fills **eight pairs of strings**; a holder for text that was not yet translated (name only, low).

### 7. Empty and background screens

| Class | Layout | Notes |
|---|---|---|
| `CMSUnderlay` | `cmsunderlay` (1 `Background` gadget) | Used as the underlay of **75 screens**. `PostCreate` is empty (a 1-byte function). |
| `CMSOverlay` | `cmsoverlay` (an online status bar, an online notification) | Overlay of 73 screens; `PostCreate` empty. The two gadgets show the Xbox Live state and notifications. |
| `CMSMainMenuUnderlay` | `cmsmainmenuunderlay` (1 `MainMenuBackground`) | `PostCreate` empty; the main menu's own background. |
| `CChapterListMenuScreen` | `cchapterlistmenuscreen` (4: image, label, button bar, `ChapterList`) | No method: the list gadget and the cheat screen's link `lnkChapters` do the work (unlock of levels by chapter: T01-206). |
| `CHighScoresMenuScreen`, `CHighScoresSelectLevelMenuScreen`, `CHighScoresLevelRankingsMenuScreen` | `chighscoresmenuscreen`, `chighscoresselectlevelmenuscreen`, `chighscoreslevelrankingsmenuscreen` (4, 8, 3 gadgets) | No method. The layouts form a chain by links (`lnkSelectLevel`, `lnkselect`) but **no screen opens the first one** (`menus-tables.md`: entry point) and the data holds **no high score storage**: unused (T01-206). |
| `CUpdateHighScoresMenuScreen`, `CViewHighScoresMenuScreen` | none | No method, no layout. |

## Invariants

1. The page covers 25 classes with 25 x 7 standard + 114 named = **289 exported functions**; 16 of them have named methods (script, `EXPORTS`).
2. `CMainMenu`'s four buttons are the four `btn...` gadgets of `cmainmenu` and each has exactly the handlers listed (script).
3. `CCheatsMenuScreen`'s `Step` table lists exactly the nine buttons of the layout plus `lnkChapters` (script and read).
4. The in-game menu closes on the **release** of the back command, not on the press (read; one function pair).
5. A popup answer is named `<message>_On<Result>` and the message is often a layout button: `btnQuit_OnYes`, `btnContinuePlaying_OnYes`, `btnRestartChapter_OnYes`, `btnRestartLevel_OnYes` name buttons of `cmsingamemenu`; `btnQuit_OnYes` also names `cmainmenu`'s button (script: 18 of the 40 `OnYes` handlers of the whole family name a gadget of their own layout; `btnDeletedDamagedProfiles`, `btnDeletedProfile`, `btnChangeMod` are not gadgets).

## Open questions

1. The in-game menu's enable matrix (which gadget needs which state) and the exact `OnSetCurrent` focus choice: T01-198.
2. The effects and the level numbers of the cheats; whether the cheat level is `sam_iEnableCheats`: T01-198.
3. Profile storage details (the damaged and dropped profile counters, the maximum `plp_iMaxPlayerProfiles`, where the profile screens write): T01-199.
4. Which screen the main menu's `btnMultiplayer` popup answers (`GamespyConnect_OnSuccess`) are delivered to: the class has none, `CMSMultiplayer` has; the popup returns to the screen that opened it: T01-197.
5. The unused screens (high scores, chapter list, assign profile, profiles): T01-206.
6. `CMSTempTranslation` and `CMSDebugSettings` details: T01-206.

## Test vectors

1. **Main menu start-up** (read): fresh start, one damaged profile, no dropped profiles, game options loaded. First accept command: default login (if allowed), then the YES/NO popup about **1** damaged profile (`%1` = 1); the command is consumed; after YES (`btnDeletedDamagedProfiles_OnYes`) the second accept command passes the other two checks and activates the focused gadget.
2. **In-game menu back key** (read): `OnSetCurrent` clears the flag; command 12 pressed sets it; command 12 released with the flag set closes the menu. A release of 12 without a press on this screen (the key that opened the menu) therefore does not close it.
3. **Cheats enable** (read): cheat level 1 enables the five level-1 gadgets and disables `lnkChapters`, `btnTurboMode`, `btnKillAllEnemies`, `btnToggleHud`, `btnToggleGhost`; level 2 enables all ten; level 0 none (and the screen is not opened by command 21).
4. **Mod list** (read): with `Content/Foo.mod` present the list is `SeriousSam2`, `InSamnity2`, `Sam2Renovation`, `Foo`; the description of `Foo` is read from `Content/Foo.nfo`.
