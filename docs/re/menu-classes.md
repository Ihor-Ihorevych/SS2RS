# Menu screen classes of `Sam2Game.dll`: overview (T01-112, part of T01-026)

- **Roadmap task:** T01-112 (part 11 of the class inventory T01-026; the other parts are T01-102 to T01-113). Pages of this part: [`menu-classes-shell.md`](menu-classes-shell.md) (main menu, profiles, in-game menu, cheats, extras, mods, the empty and underlay screens), [`menu-classes-play.md`](menu-classes-play.md) (single player flow, level lists, save, load and demo screens, statistics, multiplayer character), [`menu-classes-options.md`](menu-classes-options.md) (video, audio, game, network, controls; the quality presets), [`menu-classes-session.md`](menu-classes-session.md) (multiplayer, session settings, votes, server lists), [`menu-classes-online.md`](menu-classes-online.md) (Xbox Live and online friends screens), [`menu-classes-popups.md`](menu-classes-popups.md) (popups, the loading screen, the popup protocol), [`menu-gadget-members.md`](menu-gadget-members.md) (the gadget classes and their declared members).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/menu_classes_verify.py` reproduces from the data and the export names (the 104 classes and 1,412 functions, which layout belongs to which class, the match of handler names to gadget names, the kinds of gadget each event belongs to, the member names of 22 gadget classes against the data's type tables, the option lists that equal the tables read from the binary); **medium** for everything read from the binary once (rules marked "read") and for the class tree and the base interface (constructor chains and virtual function tables, two readings of different things); **low** for "(name)" purposes and for the Open questions. Nothing was run.

Prerequisites: `gui.md` and `menus-tables.md` (the 99 layouts, 845 gadgets, link graph, platform masks), `rsc-ctl.md` (the container and gadget classes), `localization.md` (`CTransString`), `console-variables.md` (the variables the options screens write), `replication.md` (net behaviour), and the sibling parts of this inventory (`players-session-classes.md`, `weapon-classes.md`, `worldcontent-classes.md`).

## Summary

`Sam2Game.dll` exports **104 menu classes**: one per screen of the game's menu system (`CMainMenu`, the `CMS*` screens, the `CPopupMenu*` popups, the underlays, the high score and match list screens). They export **1,412 functions**: **728** are the standard seven every class has (two constructors, destructor, two operators, one special function, `mdGetDataType`) and **684** are named methods. **None of the 104 is a saved class**: a screen is a layout file of the data (`Menus/*.rsc`, 99 files, `gui.md`) that names, by a string, the class that runs it. The layouts therefore are the **data side** of these classes: they decide which gadgets a screen has and where; the classes decide what the gadgets do.

| Fact | Value | Basis |
|---|---|---|
| Exported menu classes / functions | 104 / 1,412 (728 standard + 684 named) | script, section 1 |
| Classes with no named method at all | 25 (screens that are pure layout, or unused) | script |
| Classes with a layout file of the same name | **95** of 104 (stem equals the class name, case-folded); 9 have no layout | script, section 2 |
| Layouts with no class of that name | 4 (`ccreditsmenuscreen`, `cloadgamemenuscreen`, `cmatchlistmenuscreen`, `coptionsmenuscreen`) | script |
| Screen class written in the layout equals the C class | 61; its S twin 22; another name 12 | script |
| Handler methods `<name>_On<Event>` | 372 (122 virtual, 250 plain); **263** name a gadget of their own layout; 82 are popup answers, 12 list-item events, 15 name a gadget the layout does not have (7 of them in the one class without a layout, which has 8 handlers) | script, export names |
| Base class | one non-exported base, `CMenuScreen`, for all 104 (two screens derive from `CPopupMenuPleaseWait`) | profiled, medium |
| Base interface | 35 virtual functions | vtables, medium |
| Net behaviour | **none**: menu screens are not entities and are not replicated; the settings screens write the host's session parameters directly | export names (no `GetNetBehavior` in 104) |

**Headline findings.**

1. **A screen is found by the file name of its layout, and the layout names the class.** The code opens screens with strings like `CMSSinglePlayer`, `CPopupMenuYesNoLarge`, `SPopupMenuYesNo`: these equal the **file stems** of the layouts (case-folded; 95 of the 104 classes have a layout of exactly that stem), while the class written **inside** the layout (`CMenuScreenLayout.1`) is often another name (`SSingleplayerMenuScreen` in `cmssingleplayer.rsc`, `SInGameMenu` in two files). The relation is therefore stem to layout and layout to class, not class to layout (section 5).
2. **Handlers are bound by name.** A method `btnQuit_OnClick` handles the event `OnClick` of the gadget named `btnQuit`. All 263 gadget handlers match a gadget of their own layout; the event suffix fixes the gadget kind (**every** `_OnChange` belongs to an option, slider or integer select box, 105 of 105; every `_OnClick` to a button, image button or name entry; `_OnSelect` and `_OnTargetHit` to lists). Of the other 109, 82 are not gadget handlers but **answers**: a screen opens a popup with a message name and gets `<message>_OnYes`, `_OnNo`, `_OnOk`, `_OnAccept`, `_OnCancel`, `_OnSuccess` or `_OnFailed` back (another 30 answers carry the name of the button that opened the popup and are counted among the 263); 12 are events of **list items** (`LoadLevelItem_OnSelect`); 15 name a gadget the layout does not contain (section 4).
3. **Settings screens use two small tables the data confirms.** The 18-entry enemy-multiplier list (50% to 1,000%) is a table in the code and the same 18 texts in the layout; the five difficulty names map to difficulty values 1 to 5; the video quality screen has 5 preset levels (10 to 14 for the texture size exponent and so on, 15 variables, `menu-classes-options.md`) and a sixth option `CUSTOM`; the connection screen has 3 bandwidth presets (3,000/12,000; 5,000/65,536; 7,500/100,000) and `CUSTOM`.
4. **Some code has no gadget.** The cheats screen has handlers for 11 buttons but its layout holds 9: `btnRefreshArmor` and `btnToggleFly` are in the code only. The Renovation options screen exists as a class with 8 handlers and **has no layout in this install**. Nine classes have no layout (section 3).
5. **The binary declares the member names of the gadget classes.** 22 of 25 gadget and menu data classes match the binary's declared names by count, order and type (section 6, `menu-gadget-members.md`): for example `CMenuGadgetLayout.1` is `mgl_idName` and `.7` is `mgl_plaPlatform`, which confirms `gui.md`'s platform mask.

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were listed; the 932 exported classes were grouped by name (T01-026 split, group "P10 menus"). The 104 classes export 1,412 functions; names are quoted as exported. The three other modules were searched for exported menu classes: `EngineGui.dll` exports only the **editor's** menu classes (an editor menu filler and similar); `Engine.dll` and `Sam2GameGui.dll` export none. **The runtime menu base classes (`CMenuScreen`, the gadget classes) are not exported by any module**; they live inside `Sam2Game.dll` and are known from member tables and constructor chains only.
- **Profiles.** All **684 named methods** were profiled: the strings each refers to (gadget names, screen names, translation keys) and the exported functions it calls. This is the **"profiled"** basis. About **150 were decompiled and read** (the **"read"** basis: the popup and loading classes, the main menu, in-game menu, cheats, difficulty, save and load, video and network options, the multiplayer settings, statistics); the others were looked at for size and strings only. Identical small functions are shared by the linker, so the decompiler shows one class's name for several; such names were not used (for example the plain `PostCreate` of eight classes and the `OnCancel` pair).
- **Constructors and virtual tables.** The copy constructor of each of the 104 classes was read for its parent constructor call; the virtual function table of each was compared slot by slot (35 to 57 slots).
- **Member tables.** The binary's member tables (name, type, flags) of the gadget layout classes, the menu parameters and the runtime gadgets were read (names only; offsets are not recorded, `conventions.md`).
- **Data.** `tools/re/menu_classes_verify.py` (a fraction of a minute) reads the 99 layouts and prints sections 1 to 6 of this page and the pages that follow. It contains as constants the **export names** of the 104 classes, the **declared member names** of the gadget classes and the **numbers read once from the binary** (the tables above); everything else is computed.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run (no screen was opened, no input sent); the gadget classes' own behaviour (focus, repeat, drawing); the exact enable rules of the in-game menu; the cvar each options gadget writes (parked, T01-196); the Xbox Live and online screens beyond their handler lists; the behaviour of the 33 classes with no handler of their own beyond their layouts.

## Layout / rules

### 1. Basis marks used on the six pages

| Mark | Means |
|---|---|
| **read** | the function was decompiled and read; the rule is medium |
| **profiled** | the strings and callees of the function were listed; the purpose sentence follows from them and the name; medium to low |
| **name** | the name only; low |
| **data** | counted or compared with the layouts by the script; high |

Confidence words follow `conventions.md` section 3. Names ending in `(ours)` are invented; every other identifier is the binary's own, quoted as exported or declared. "Slot" is the position in a virtual function table, counted from 0.

### 2. The class tree and the base interface

**Tree (medium).** The constructors of all 104 classes call one and the same non-exported base constructor, `CMenuScreen` (the binary's member table of `SMSControllerOptions` names `CMenuScreen` as its parent: a second, independent reading). Two classes call an exported constructor instead: **`CMSLoading` and `CPopupMenuPleaseWaitLarge` derive from `CPopupMenuPleaseWait`**. There is no other inheritance among the 104: for example `CPopupMenuYesNoLarge` and `CPopupMenuYesNoTimed` are not children of `CPopupMenuYesNo`. The base holds the gadget list of the screen, the overlay and underlay names, the current gadget and the **caller gadget identifier** (`ms_idCallerGadget`, the one base member with a declared name).

**Virtual interface (35 slots; medium).** Every class has the same first 35 slots. Names are those of the exported overrides; "use" says what the callers of the slot do.

| Slot | Name | Exported overrides | Use (read) |
|---|---|---|---|
| 0 | `mdGetDataType` | all 104 | the class's type information |
| 1 | destructor | all | - |
| 4 | (find gadget by identifier) | none | returns the gadget object of a name; the caller checks its kind with a type test |
| 6 | (set the current gadget) | none | focuses a gadget by name |
| 13 | (enable or disable a gadget) | none | name and a flag; used by the in-game menu and the session settings |
| 18 | `IsInteractive` | `CPopupMenuPleaseWait`, `CMSLoading` (both return 0) | whether input reaches the screen |
| 19 | `ExitMenuScreen` | `CMSDebugSettings` | closes the screen and returns to the previous one; called by many handlers |
| 20 | (raise a named screen event) | none | `OnYes`, `OnNo`, `OnOk`, `OnCancel`, `OnAccept`, `OnSuccess`, `OnFailed` are raised through it (section 4) |
| 21 | `OnSetCurrent` | 50 | the screen has become the top screen |
| 22 | `OnUnsetCurrent` | 11 | another screen covers it |
| 23 | `PostCreate` | 60 | after the gadgets exist |
| 25 | `OnAnyCommandPressed` | `CMSXboxLivePassEntry` | every command, before the screen's own |
| 26 | `OnCommandPressed` | 34 | a menu command (back, accept, a cheat key) |
| 27 | `OnCommandReleased` | `CMSInGameMenu` | release of a command |
| 33 | `Step` | 25 | every frame while the screen is up |
| 34 | `Render` | 4 (`CMainMenu`, `CMSLoadGame`, `CMSControllerOptions`, `CMSMultiCharDesc`) | extra drawing after the gadgets |

The other slots (2, 3, 5, 7 to 12, 14 to 17, 24, 28 to 32) are base functions that no menu class overrides by an exported function (a class may still override them with an internal one). **Handlers are further slots**: the first handler of a class is slot 35. Of the 372 handler methods **122 are virtual** (they take a slot) and **250 are ordinary member functions** (by the export names). Of the 67 classes that have handler methods, 32 have only plain ones (for example `CMSVideoOptions` with 17 and `CMSWeaponSettings` with 14), 23 only virtual ones and 12 both. Slot counts: 63 classes have exactly 35 slots, 41 have 36 to 57 (`CMSFriendsOptions` 57, `CMSInGameMenu` 46, `CMSSelectDifficulty` 45, `CCheatsMenuScreen` 45).

**Menu commands (read, medium).** `OnCommandPressed` receives a command number. Four values occur in the code: **12** is "back": popups swallow it (return 1 and do nothing), the in-game menu records it on press and closes on release, the difficulty screen closes or goes back; **11** and **31** are the "accept/start" commands: the main menu reacts to them once (the **initial interaction**, below); **21** is the **cheat key**: when the global cheat level is non-zero the main menu, the in-game menu and the difficulty screen open `CCheatsMenuScreen` (the level is probably `sam_iEnableCheats`: `console-config.md`; the binding is not established). The command enumeration (`MenuCommand`) has no names in the data: T01-197.

### 3. Which layout belongs to which class (data)

The script finds the layout of each class by **file stem**. Result (`tools/re/menu_classes_verify.py` sections 2 and 7):

- **95 classes have a layout of the same stem**; in 61 of them the class written inside the layout is the C class itself, in 22 it is the **S twin** (`CMSProfileSelection` is run by `SMSProfileSelection`), in 12 another name: `CMSCallAVote` (`SInGameMenu`, the same class as the in-game menu), `CMSCooperativeItemSettings` (`CMSCooperativeSettings`), `CMSInGameMenu` (`SInGameMenu`), `CMSMultiplayerCharacter` (`SMultiplayCharacterMenuScreen`), `CMSOptiMatchDetails` (`CMSOptimatchDetails`: a difference in case), the three popup underlays (`SPopupMenuUnderlay`), `CMSSessionSettings` (`CMSessionSettings`), `CMSSinglePlayer` (`SSingleplayerMenuScreen`), `CPopupMenuAbortGame` (`SPopupMenuOk`) and `CPopupMenuAbortGameLarge` (`CPopupMenuOkLarge`).
- **9 classes have no layout** in this install: `CAssignNewProfileMenuScreen`, `CGameOptionsMenuScreen`, `CMSRenovationOptions`, `CMSTempTranslation`, `CMatchListDetailMenuScreen`, `CProfilesMenuScreen`, `CSLMatchListMenuScreen`, `CUpdateHighScoresMenuScreen`, `CViewHighScoresMenuScreen`. `CMatchListDetailMenuScreen` is the target of the one unresolved link of `gui.md` (`cmatchlistmenuscreen` links to `SMatchListDetailMenuScreen`, which has no layout); the others are screens the game defines but does not ship, or ships under another name.
- **4 layouts have no class of the same stem**: `ccreditsmenuscreen` (a second layout of the class `CMSExtras`, the credits), `cloadgamemenuscreen` (class `SLoadGameMenuScreen`), `cmatchlistmenuscreen` (`SMatchListMenuScreen`), `coptionsmenuscreen` (`SOptionsMenuScreen`). Whether they belong to `CMSLoadGame`, `CSLMatchListMenuScreen`, `CGameOptionsMenuScreen` is not established (T01-205).
- **Six screen classes have more than one layout** (`CMSExtras` 2, `SInGameMenu` 2, `CMSCooperativeSettings` 2, `SPopupMenuUnderlay` 3, `SPopupMenuOk` 2, `CPopupMenuOkLarge` 2); in these cases the **file stem** (what the code opens) selects the layout and the class is shared. The script counts **92 distinct screen classes** where `gui.md` says 93; the layouts are the same (99, 6 classes with 2 or 3), so one of the two counts is off by one (T01-205).

### 4. Handlers and the popup protocol

- **Gadget handlers (data).** `<gadget>_On<Event>`: **263** of the 372 name a gadget of the class's own layout. Events against the gadget kinds of the layouts: `OnClick` 117 (88 on buttons, 25 on image buttons, 4 on name entries), `OnChange` 105 (85 option select boxes, 10 slider boxes, 10 integer select boxes: **no other gadget kind has an `OnChange`**), `OnSetFocus` 7 (5 buttons, 2 option boxes), `OnLoseFocus` 2 (option boxes), `OnYes` 18 and `OnNo` 7 (buttons: answers named after the opening button, below), `OnCancel` 5 (4 name entries, 1 button), `OnEscape` 1 (a name entry), `OnTargetHit` 1 (a scroll list). The gadget name prefix fixes the kind: `btn` (119 handlers), `opsl` option box (89), `ibtn` image button (24), `sld` slider box (10), `isl` integer box (10), `ene` name entry (7), `et` text entry (2), `scl` scroll list (1), `imgb` image button (1).
- **Handlers without a gadget (15; data).** `CCheatsMenuScreen`: `btnRefreshArmor`, `btnToggleFly` (the layout has no such buttons); `CMSAudioOptions`: `sldVoice` (no such slider; the voice slider is `sldVoiceVolume` in `CMSXboxLiveOptions`); `CMSInGameMenu`: `btnResume`; `CMSSinglePlayer`: `btnCustomLevel` and `btnNewCampaign` (the layout opens the same screens through the links `lnkCustomLevel` and `lnkStartNewCampaign`); `CMSModSelection` and `CMSProfileSelection`: the **item click** of their scroll lists (`sclModItem`, `sclProfileItem`); `CMSRenovationOptions`: 7 (the class has no layout; with its answer `btnDefaultSettings_OnYes` it has 8 handlers).
- **List-item events (12).** `LoadLevelItem_OnSelect`, `SaveLevelItem_OnSelect` and `_OnNewSaveGame`, `RecordDemoItem_OnSelect` and `_OnNewRecordDemo`, `PlayDemoItem_OnSelect`, `SelectLevelItem_OnSelect` (two classes), `SelectEpisodeItem_OnSelect`, `SelectMovieItem_OnSelect`, `MultiCharItem_OnSelect`, `PlayerList_OnSelect`: the prefix is the name of the **list item kind** (`menu-classes-play.md`), not of a gadget.
- **Answers (read, medium).** A screen that wants a decision calls the base function that **opens a popup** with four arguments: the popup screen's name (`SPopupMenuYesNo`, `SPopupMenuOk`, `CPopupMenuYesNoLarge`, `CPopupMenuEnterText`, `CPopupMenuPleaseWait`, ...), a **message name**, the text and (for yes and no) the name of the button to focus (`btnNo`). The message name is usually the name of the button that opened the popup (`btnQuit`, `btnApply`, `btnDefaultSettings`) or a task name (`Overwrite`, `NewSaveGame`, `StopGame`, `GamespyConnect`). When the popup ends it **raises an event on the screen below it** and that screen's method `<message>_On<Result>` runs. The popup classes raise: `CPopupMenuYesNo` `OnYes` and `OnNo`; `CPopupMenuOk` `OnOk`; `CPopupMenuCancel` `OnCancel`; `CPopupMenuEnterText` `OnAccept` (after storing the typed text in a variable called `EnteredText`) and `OnCancel`; `CPopupMenuPleaseWait` `OnSuccess` or `OnFailed`; `CPopupMenuYesNoTimed` `OnNo` when its time is up. **112 handlers are such answers** (`OnYes` 40, `OnNo` 26, `OnFailed` 12, `OnSuccess` 12, `OnAccept` 9, `OnCancel` 7, `OnOk` 6): 30 carry the name of a gadget of the layout (`btnApply_OnYes`: the message is the button that opened the popup) and 82 another name (`Overwrite_OnAccept`, `GamespyConnect_OnFailed`).
- **Screen variables.** Screens pass values by name through a small variable store that outlives the screen: `SelectedLevel`, `SelectedEpisode`, `SelectedCharacter`, `SelectedPlayer`, `SelectedFriendName`, `OnlineMessageType`, `VoicePlaybackPurpose`, `EnteredText`, `PlayerSelected`, `LevelFinished` (strings in the profiles; read for the level, character and episode lists).

### 5. Opening a screen

A screen is opened by the base function that takes **the name of a layout (its file stem)** and a flag, and returns the new screen; the caller may then set the new screen's **caller gadget identifier**. Examples (read): the main menu's `btnMultiplayer` opens `CMSProfileSelection` after the online connection popup succeeds and passes `btnInternet`; `btnSystemLink_OnYes` opens `CMSProfileSelection` with `btnSystemLink`. **The profile selection decides the next screen from the caller gadget** (`CMSProfileSelection::sclProfileItem_OnClick`, profiled): `lnkSinglePlayer` leads to `CMSSinglePlayer`, `lnkPlayLive` to `CMSXboxPlayLive`, `lnkProfileSettings` to `SProfileOptionsMenuScreen`, `btnInternet` to `CMSGamespy`, `btnLocalNetwork` and `btnSystemLink` to `CMSServerList`. The data agrees: the three link gadgets that open the profile selection are exactly `lnkSinglePlayer` and `lnkProfileSettings` (main menu) and `lnkPlayLive` (Xbox Live menu) (`menus-tables.md`). `CMSSelectDifficulty::Proceed` uses the same device with `lnkMultiplayer`, `lnkProfileSettings` and `lnkSinglePlayer`. Other gadgets (`lnk...`) are opened by the base class without a handler: the 25 classes with no named method, such as `CMSOptionsSelection`, are pure layouts with links.

### 6. The 104 classes

Columns: **Page** where the class is described; **Layout** the file of that stem; **Screen class** the class named inside the layout (`gui.md`: `CMenuScreenLayout.1`); **Gadgets** its gadget count; **Functions (named)** exported functions in all and named methods among them. "-" = no layout.

| Class | Page | Layout | Screen class | Gadgets | Functions (named) |
|---|---|---|---|---|---|
| `CMainMenu` | shell | `cmainmenu` | `CMainMenu` | 14 | 27 (20) |
| `CMSExtras` | shell | `cmsextras` | `CMSExtras` | 8 | 10 (3) |
| `CMSAchievements` | shell | `cmsachievements` | `CMSAchievements` | 9 | 13 (6) |
| `CMSModSelection` | shell | `cmsmodselection` | `CMSModSelection` | 6 | 13 (6) |
| `CMSProfileSelection` | shell | `cmsprofileselection` | `SMSProfileSelection` | 6 | 15 (8) |
| `CMSCreateNewProfile` | shell | `cmscreatenewprofile` | `SMSCreateNewProfile` | 4 | 14 (7) |
| `CMSChangeName` | shell | `cmschangename` | `SMSChangeName` | 4 | 11 (4) |
| `CMSEnterUnlockKey` | shell | `cmsenterunlockkey` | `SMSEnterUnlockKey` | 5 | 11 (4) |
| `CProfileOptionsMenuScreen` | shell | `cprofileoptionsmenuscreen` | `SProfileOptionsMenuScreen` | 8 | 10 (3) |
| `CAssignNewProfileMenuScreen` | shell | - | - | - | 7 (0) |
| `CProfilesMenuScreen` | shell | - | - | - | 7 (0) |
| `CMSInGameMenu` | shell | `cmsingamemenu` | `SInGameMenu` | 17 | 28 (21) |
| `CCheatsMenuScreen` | shell | `ccheatsmenuscreen` | `SCheatsMenuScreen` | 14 | 21 (14) |
| `CMSDebugSettings` | shell | `cmsdebugsettings` | `CMSDebugSettings` | 77 | 17 (10) |
| `CMSDemoEnd` | shell | `cmsdemoend` | `CMSDemoEnd` | 4 | 12 (5) |
| `CMSTempTranslation` | shell | - | - | - | 7 (0) |
| `CMSUnderlay` | shell | `cmsunderlay` | `CMSUnderlay` | 1 | 8 (1) |
| `CMSOverlay` | shell | `cmsoverlay` | `CMSOverlay` | 2 | 8 (1) |
| `CMSMainMenuUnderlay` | shell | `cmsmainmenuunderlay` | `CMSMainMenuUnderlay` | 1 | 8 (1) |
| `CChapterListMenuScreen` | shell | `cchapterlistmenuscreen` | `SChapterListMenuScreen` | 4 | 7 (0) |
| `CHighScoresMenuScreen` | shell | `chighscoresmenuscreen` | `SHighScoresMenuScreen` | 4 | 7 (0) |
| `CHighScoresLevelRankingsMenuScreen` | shell | `chighscoreslevelrankingsmenuscreen` | `SHighScoresLevelRankingsMenuScreen` | 3 | 7 (0) |
| `CHighScoresSelectLevelMenuScreen` | shell | `chighscoresselectlevelmenuscreen` | `SHighScoresSelectLevelMenuScreen` | 8 | 7 (0) |
| `CUpdateHighScoresMenuScreen` | shell | - | - | - | 7 (0) |
| `CViewHighScoresMenuScreen` | shell | - | - | - | 7 (0) |
| `CMSSinglePlayer` | play | `cmssingleplayer` | `SSingleplayerMenuScreen` | 8 | 11 (4) |
| `CMSSelectDifficulty` | play | `cmsselectdifficulty` | `SMSSelectDifficulty` | 10 | 24 (17) |
| `CMSSelectLevel` | play | `cmsselectlevel` | `SMSSelectLevel` | 6 | 14 (7) |
| `CMSSinglePlayerSettings` | play | `cmssingleplayersettings` | `CMSSinglePlayerSettings` | 19 | 24 (17) |
| `CMSLoadGame` | play | `cmsloadgame` | `CMSLoadGame` | 12 | 12 (5) |
| `CMSSaveGame` | play | `cmssavegame` | `CMSSaveGame` | 5 | 15 (8) |
| `CMSPlayDemo` | play | `cmsplaydemo` | `CMSPlayDemo` | 6 | 10 (3) |
| `CMSRecordDemo` | play | `cmsrecorddemo` | `CMSRecordDemo` | 6 | 14 (7) |
| `CMSStatistics` | play | `cmsstatistics` | `CMSStatistics` | 31 | 14 (7) |
| `CMSMultiplayerCharacter` | play | `cmsmultiplayercharacter` | `SMultiplayCharacterMenuScreen` | 6 | 14 (7) |
| `CMSMultiCharDesc` | play | `cmsmultichardesc` | `CMSMultiCharDesc` | 7 | 11 (4) |
| `CMSSelectEpisodeMovies` | play | `cmsselectepisodemovies` | `CMSSelectEpisodeMovies` | 6 | 12 (5) |
| `CMSSelectMovie` | play | `cmsselectmovie` | `CMSSelectMovie` | 6 | 12 (5) |
| `CMSOptionsSelection` | options | `cmsoptionsselection` | `CMSOptionsSelection` | 7 | 7 (0) |
| `CMSVideoOptions` | options | `cmsvideooptions` | `CMSVideoOptions` | 15 | 42 (35) |
| `CMSAdvancedVideoOptions` | options | `cmsadvancedvideooptions` | `CMSAdvancedVideoOptions` | 30 | 40 (33) |
| `CMSAudioOptions` | options | `cmsaudiooptions` | `CMSAudioOptions` | 14 | 20 (13) |
| `CMSGameOptions` | options | `cmsgameoptions` | `CMSGameOptions` | 14 | 21 (14) |
| `CMSNetworkOptions` | options | `cmsnetworkoptions` | `CMSNetworkOptions` | 7 | 17 (10) |
| `CMSAdvancedOptions` | options | `cmsadvancedoptions` | `CMSAdvancedOptions` | 22 | 14 (7) |
| `CMSControllerOptions` | options | `cmscontrolleroptions` | `SMSControllerOptions` | 29 | 18 (11) |
| `CMSCustomizeKeys` | options | `cmscustomizekeys` | `SMSCustomizeKeys` | 6 | 11 (4) |
| `CMSCustomizeButtons` | options | `cmscustomizebuttons` | `SMSCustomizeButtons` | 6 | 11 (4) |
| `CMSRenovationOptions` | options | - | - | - | 18 (11) |
| `CGameOptionsMenuScreen` | options | - | - | - | 7 (0) |
| `CMSMultiplayer` | session | `cmsmultiplayer` | `CMSMultiplayer` | 5 | 16 (9) |
| `CMSGamespy` | session | `cmsgamespy` | `CMSGamespy` | 10 | 16 (9) |
| `CMSServerList` | session | `cmsserverlist` | `SMSServerList` | 4 | 9 (2) |
| `CMSQuickMatch` | session | `cmsquickmatch` | `SMSQuickMatch` | 7 | 11 (4) |
| `CMSOptiMatch` | session | `cmsoptimatch` | `CMSOptiMatch` | 10 | 9 (2) |
| `CMSOptiMatchDetails` | session | `cmsoptimatchdetails` | `CMSOptimatchDetails` | 5 | 10 (3) |
| `CMSOptiMatchList` | session | `cmsoptimatchlist` | `CMSOptiMatchList` | 5 | 8 (1) |
| `CMSSessionSettings` | session | `cmssessionsettings` | `CMSessionSettings` | 15 | 20 (13) |
| `CMSCooperativeSettings` | session | `cmscooperativesettings` | `CMSCooperativeSettings` | 21 | 26 (19) |
| `CMSCooperativeItemSettings` | session | `cmscooperativeitemsettings` | `CMSCooperativeSettings` | 8 | 17 (10) |
| `CMSDeathmatchSettings` | session | `cmsdeathmatchsettings` | `CMSDeathmatchSettings` | 8 | 13 (6) |
| `CMSItemSettings` | session | `cmsitemsettings` | `CMSItemSettings` | 15 | 24 (17) |
| `CMSPowerupSettings` | session | `cmspowerupsettings` | `CMSPowerupSettings` | 8 | 17 (10) |
| `CMSWeaponSettings` | session | `cmsweaponsettings` | `CMSWeaponSettings` | 18 | 27 (20) |
| `CMSVotingSettings` | session | `cmsvotingsettings` | `CMSVotingSettings` | 8 | 17 (10) |
| `CMSSelectSessionLevel` | session | `cmsselectsessionlevel` | `SMSSelectSessionLevel` | 6 | 12 (5) |
| `CCreateMatchMenuScreen` | session | `ccreatematchmenuscreen` | `SCreateMatchMenuScreen` | 0 | 7 (0) |
| `CMatchListDetailMenuScreen` | session | - | - | - | 7 (0) |
| `CSLMatchListDetailMenuScreen` | session | `cslmatchlistdetailmenuscreen` | `SSLMatchListDetailMenuScreen` | 9 | 7 (0) |
| `CSLMatchListMenuScreen` | session | - | - | - | 7 (0) |
| `CMSCallAVote` | session | `cmscallavote` | `SInGameMenu` | 6 | 13 (6) |
| `CMSPlayerList` | session | `cmsplayerlist` | `CMSPlayerList` | 4 | 11 (4) |
| `CMSXboxLive` | online | `cmsxboxlive` | `CMSXboxLive` | 11 | 18 (11) |
| `CMSXboxLiveOptions` | online | `cmsxboxliveoptions` | `CMSXboxLiveOptions` | 6 | 13 (6) |
| `CMSXboxLivePassEntry` | online | `cmsxboxlivepassentry` | `CMSXboxLivePassEntry` | 2 | 13 (6) |
| `CMSXboxPlayLive` | online | `cmsxboxplaylive` | `CMSXboxPlayLive` | 6 | 8 (1) |
| `CMSOnlineLogin` | online | `cmsonlinelogin` | `CMSOnlineLogin` | 3 | 25 (18) |
| `CMSOnlineAcceptGameInvite` | online | `cmsonlineacceptgameinvite` | `CMSOnlineAcceptGameInvite` | 4 | 26 (19) |
| `CMSOnlineFriendList` | online | `cmsonlinefriendlist` | `CMSOnlineFriendList` | 4 | 7 (0) |
| `CMSOnlinePlayerList` | online | `cmsonlineplayerlist` | `CMSOnlinePlayerList` | 4 | 7 (0) |
| `CMSOnlineMessageFeedback` | online | `cmsonlinemessagefeedback` | `CMSOnlineMessageFeedback` | 6 | 10 (3) |
| `CMSOnlineSendMessage` | online | `cmsonlinesendmessage` | `CMSOnlineSendMessage` | 9 | 15 (8) |
| `CMSOnlineVoicePlayback` | online | `cmsonlinevoiceplayback` | `CMSOnlineVoicePlayback` | 7 | 23 (16) |
| `CMSFriendsOptions` | online | `cmsfriendsoptions` | `CMSFriendsOptions` | 16 | 32 (25) |
| `CMSPlayerOptions` | online | `cmsplayeroptions` | `CMSPlayerOptions` | 10 | 14 (7) |
| `CMSPlayerFeedback` | online | `cmsplayerfeedback` | `CMSPlayerFeedback` | 7 | 10 (3) |
| `CMSPlayerComplaint` | online | `cmsplayercomplaint` | `CMSPlayerComplaint` | 9 | 13 (6) |
| `CPopupMenuAbortGame` | popups | `cpopupmenuabortgame` | `SPopupMenuOk` | 2 | 8 (1) |
| `CPopupMenuAbortGameLarge` | popups | `cpopupmenuabortgamelarge` | `CPopupMenuOkLarge` | 2 | 7 (0) |
| `CPopupMenuCancel` | popups | `cpopupmenucancel` | `SPopupMenuCancel` | 2 | 12 (5) |
| `CPopupMenuCancelLarge` | popups | `cpopupmenucancellarge` | `CPopupMenuCancelLarge` | 2 | 7 (0) |
| `CPopupMenuEnterText` | popups | `cpopupmenuentertext` | `CPopupMenuEnterText` | 3 | 14 (7) |
| `CPopupMenuEnterTextLarge` | popups | `cpopupmenuentertextlarge` | `CPopupMenuEnterTextLarge` | 3 | 7 (0) |
| `CPopupMenuOk` | popups | `cpopupmenuok` | `SPopupMenuOk` | 2 | 12 (5) |
| `CPopupMenuOkLarge` | popups | `cpopupmenuoklarge` | `CPopupMenuOkLarge` | 2 | 7 (0) |
| `CPopupMenuPleaseWait` | popups | `cpopupmenupleasewait` | `CPopupMenuPleaseWait` | 5 | 17 (10) |
| `CPopupMenuPleaseWaitLarge` | popups | `cpopupmenupleasewaitlarge` | `CPopupMenuPleaseWaitLarge` | 3 | 7 (0) |
| `CPopupMenuYesNo` | popups | `cpopupmenuyesno` | `CPopupMenuYesNo` | 3 | 13 (6) |
| `CPopupMenuYesNoLarge` | popups | `cpopupmenuyesnolarge` | `CPopupMenuYesNoLarge` | 3 | 7 (0) |
| `CPopupMenuYesNoTimed` | popups | `cpopupmenuyesnotimed` | `CPopupMenuYesNoTimed` | 3 | 9 (2) |
| `CMSPopupMenuUnderlay` | popups | `cmspopupmenuunderlay` | `SPopupMenuUnderlay` | 1 | 8 (1) |
| `CMSPopupMenuUnderlayLarge` | popups | `cmspopupmenuunderlaylarge` | `SPopupMenuUnderlay` | 1 | 7 (0) |
| `CMSPopupMenuUnderlayText` | popups | `cmspopupmenuunderlaytext` | `SPopupMenuUnderlay` | 1 | 7 (0) |
| `CMSLoading` | popups | `cmsloading` | `CMSLoading` | 24 | 23 (16) |

## Invariants

1. The export table lists **104** menu classes with **1,412** functions: 728 standard (7 per class) + 684 named (script: 104 x 7 + 684 = 1,412).
2. **Every handler that names a gadget names one of its own layout** (263 of 263; no handler names a gadget of another class's layout). Every `_OnChange` of a layout class names an option, slider or integer select box (105 of 105), every `_OnLoseFocus` an option box (2 of 2).
3. **A layout is chosen by file stem; two layouts may share one class.** An implementation that keys screens by the class written inside the layout will confuse 6 groups of layouts (section 3).
4. **No menu class is replicated.** None of the 104 exports `GetNetBehavior`; screens run on every machine with their own state (checked on the export table: 0 of 104). The settings screens of a session write the session parameters of the local machine; that they are the host's is by the flow, not by a check in the code (`menu-classes-session.md`).
5. The four popup classes with handlers (`YesNo`, `Ok`, `Cancel`, `PleaseWait`) **swallow command 12 (back)**: a popup cannot be dismissed with the back command (read, one shared function).
6. For the options screens the numbers in `menu-classes-options.md` hold: 18 enemy percentages (50 to 1,000), 5 difficulty names to 1 to 5, 5 quality levels plus `CUSTOM`, 3 connection presets plus `CUSTOM`, 4 display modes (script, the option lists equal the tables).

## Open questions

Ranked by what blocks an implementer; each names the task that will answer it (appended to the roadmap with this change).

1. **The base `CMenuScreen` and the gadget classes at run time** (focus order, command routing, the meaning of the command numbers 11, 12, 21, 31, the popup stack, how links are opened, how events are raised and found by name, how the Large popups answer without handlers): T01-197 (with T01-078, `gui.md`).
2. **Which variable each options gadget sets** (video, advanced video, audio, game, controls, profile options; the 15 quality variables are known, most others are not mapped to console variables): T01-196.
3. **The in-game menu's enable rules**, the continue-playing and restart rules, the cheat functions and cheat levels: T01-198.
4. **Profiles, saves, loads and demos**: file names, list contents, limits and deletion rules: T01-199.
5. **Level lists, loading screen and the statistics screen** (level information, thumbnails, the bonus counters): T01-200.
6. **Multiplayer**: the mapping of every session setting to the session parameters, votes, the server and match list screens: T01-201.
7. **Xbox Live and the online screens** (15 classes, 234 functions, Xbox only): T01-202. Needed only if the Xbox build is ever a target; the PC build hides their gadgets by the platform mask (`gui.md`).
8. **Popup details** (timeouts, the progress callbacks, the Large variants): T01-203.
9. **The layout classes and runtime gadget classes without a declared member table** (arranger, image button, name entry, custom keys, list items, ...): T01-204.
10. **Layout bookkeeping**: 9 classes without layout, 4 layouts without class, 92 against 93 screen classes, the Renovation options screen: T01-205.
11. **Unused screens** (the high score screens, the chapter list, the temporary translation screen, the create-match screen): are they reachable at all: T01-206.

## Test vectors

1. `CMainMenu`: layout `cmainmenu` (class `CMainMenu`, 14 gadgets, six links); named methods 20 of 27 functions: `CheckForGameInvitations`, `HandleDamagedProfiles`, `HandleDroppedProfiles`, `HandleFaultyGameOptions`, `OnCommandPressed`, `OnInitialInteraction`, `OnSetCurrent`, `PostCreate`, `Render` and 11 handlers (`btnQuit_OnClick`, `btnQuit_OnYes`, `btnMultiplayer_OnClick`, `btnSystemLink_OnClick/_OnNo/_OnYes`, `btnXboxLive_OnClick`, `btnDeletedDamagedProfiles_OnNo/_OnYes`, `btnFreeMoreSpace_OnNo`, `btnOptionsLoadFailed_OnOk`). The layout's four buttons are `btnMultiplayer`, `btnSystemLink`, `btnXboxLive` and `btnQuit`; their six handlers name them, while `btnDeletedDamagedProfiles`, `btnFreeMoreSpace` and `btnOptionsLoadFailed` are message names of popups (not gadgets).
2. `CPopupMenuYesNo`: layout `cpopupmenuyesno` (class `CPopupMenuYesNo`, gadgets `lblMessageText`, `btnYes`, `btnNo`); `btnYes_OnClick` raises `OnYes`, `btnNo_OnClick` raises `OnNo`; the caller `CMainMenu::btnQuit_OnClick` opened it with the message `btnQuit` and gets `btnQuit_OnYes` or nothing for no (no `btnQuit_OnNo` exists).
3. `opslEnemyMaxMultiplier` (two layouts: `cmscooperativesettings`, `cmssingleplayersettings`): 18 options `50%`, `75%`, `100%`, ..., `1000%`; option 2 (`100%`) is the default; the code maps option index 0 to 50, 1 to 75, 2 to 100, 3 to 125, 10 to 300, 11 to 400, 17 to 1000 and any other index to 100.
4. `opslDifficulty`: 5 options `TOURIST`, `EASY`, `NORMAL`, `HARD`, `SERIOUS`, default option 2; the code maps option 0 to difficulty 1, 1 to 2, 2 to 3, 3 to 4, 4 to 5.
