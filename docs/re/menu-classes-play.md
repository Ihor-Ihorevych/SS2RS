# Menu classes: single player flow, level lists, save, load, demos, statistics (T01-112, page 2 of 6)

- **Roadmap task:** T01-112 (overview: [`menu-classes.md`](menu-classes.md); this page covers 13 classes of the "play" group)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, layouts, gadget names, handler-to-gadget matches and the option lists (script); **medium** for rules marked "read"; **low** for "(profiled)" and "(name)" purposes and for the Open questions. Nothing was run.

Prerequisites: [`menu-classes.md`](menu-classes.md) (basis marks, popup protocol, screen opening by name and caller gadget), `player-rules.md` (difficulty values), `save-game.md` (the save format), [`menu-classes-shell.md`](menu-classes-shell.md) (profile screens).

## Summary

The play group is the path **from the profile to a running level** (single player menu, difficulty, level list, game settings), the **save, load and demo** screens that the in-game and single player menus open, the **end-of-level statistics** and the **multiplayer character** pair. 13 classes, **187 exported functions** (91 standard + 96 named), all 13 with named methods and a layout of the same stem.

## Evidence

The exported names and the layouts (`tools/re/menu_classes_verify.py`); the profiles of all 96 named methods; the decompiled bodies of `CMSSelectDifficulty` (focus, store, proceed, commands), `CMSSaveGame::CreateSaveGameFileName`, `CMSSinglePlayer`, `CMSSelectLevel::OnSetCurrent`, `CMSSinglePlayerSettings` (start, enemy multiplier), `CMSStatistics::Step`; the binary's member tables of the list item classes (names only). **Not checked:** the content of the lists (which levels, which characters, which movies), the on-disk layout of saves and demos (`save-game.md`), the statistics formulas beyond the shape of `Step`.

## Layout / rules

### 1. The single player path

```
CMainMenu --lnkSinglePlayer--> CMSProfileSelection (caller lnkSinglePlayer) --item--> CMSSinglePlayer
CMSSinglePlayer --lnkStartNewCampaign--> CMSSelectDifficulty --btn*--> (start a game)
                --lnkCustomLevel------> CMSSelectLevel --item--> CMSSinglePlayerSettings --btnStartGame--> (start)
                --lnkLoadGame (PC)----> CMSLoadGame
                --btnContinuePlaying--> (continue the profile's last game)
```

The three data links of `cmssingleplayer` are `lnkCustomLevel` (to `SMSSelectLevel`), `lnkStartNewCampaign` (to `SMSSelectDifficulty`) and `lnkLoadGame` (to `CMSLoadGame`; PC only), plus the button `btnContinuePlaying` (`menus-tables.md`). The variables `SelectedLevel` and the caller gadget carry the choice from screen to screen.

### 2. Classes

**`CMSSinglePlayer`** (layout `cmssingleplayer`, class `SSingleplayerMenuScreen`, 8 gadgets; 4 named methods).

| Method | Purpose | Basis |
|---|---|---|
| `OnSetCurrent` | takes the current profile; **if the profile has a game to continue it focuses `btnContinuePlaying`**, otherwise it prepares `lnkStartNewCampaign` and focuses that | read, low for which gadget gets the flag |
| `btnContinuePlaying_OnClick` | continues the profile's last game (two calls) | profiled |
| `btnNewCampaign_OnClick`, `btnCustomLevel_OnClick` | open `CMSSinglePlayerSettings` and prepare it (the code names `btnNewCampaign`, `btnSelectLevel`, `btnStartGame`); **the layout has no such buttons**, it has the links `lnkStartNewCampaign` and `lnkCustomLevel`, so these two handlers are not reached from this layout | read / profiled |

**`CMSSelectDifficulty`** (layout `cmsselectdifficulty`, class `SMSSelectDifficulty`, 10 gadgets: five buttons `btnTourist`, `btnEasy`, `btnNormal`, `btnHard`, `btnSerious`, the picture `imgDifficulty`, the description `lblDifficultyDesc`; 24 named methods).

- **Difficulty values (read, data).** The five buttons are difficulty **1 to 5** in this order: tourist 1, easy 2, normal 3, hard 4, serious 5. `OnSetCurrent` (802 bytes) reads the profile's difficulty, runs a 5-way switch that picks the picture (`imgDifficulty`, one texture per value), **focuses the matching button** and remembers the value in the screen. The same mapping is used by the option lists of `CMSCooperativeSettings` and `CMSSinglePlayerSettings` (`opslDifficulty`: `TOURIST`, `EASY`, `NORMAL`, `HARD`, `SERIOUS` to 1, 2, 3, 4, 5: script and read) and the order of `player-rules.md` (the table lists Tourist first).
- `btnTourist_OnSetFocus` ... `btnSerious_OnSetFocus` (5): on focus show the button's **picture and description** (`ETRSSelectDifficulty.Tourist` "For FPS beginners.", `.Easy` "For casual FPS players.", `.Normal` "For experienced FPS players.", `.Hard` "For experienced Serious Sam players.", `.Serious` "Are you serious?") and store its number as the screen's current difficulty (profiled; the stored value is what the click uses).
- `btnTourist_OnClick` ... (5): each calls **`StoreSelectedDifficulty`** then **`Proceed`**.
- **`StoreSelectedDifficulty` (read).** Writes the value into the **session parameters** (the game difficulty member of the project parameters) **and into the profile** (and marks the profile for saving); if there is no profile it logs `Could not store selected difficulty to the profile %1.`
- **`Proceed` (read, low for the mapping).** By the **caller gadget** it takes one of three ways: one starts the new game on the level in the variable `SelectedLevel` (or the default level when it is empty) and closes the screen; one returns to the main menu (`CMainMenu`: used when the caller was `lnkProfileSettings`, i.e. the player only changes the stored difficulty); one opens `SMSServerList` (the multiplayer path, caller `lnkMultiplayer`). The three caller names are `lnkSinglePlayer`, `lnkProfileSettings`, `lnkMultiplayer`; which branch belongs to which name is read from the order of the code only.
- `OnCommandPressed` (374 bytes): command 12 closes the screen (or goes back); **command 21** (cheat key) opens `CMSSinglePlayerSettings` with the `SelectedLevel` variable set (read).
- `LoadLevel_OnSuccess` (empty), `LoadLevel_OnFailed` (logs `Failed to load the level.`): the answers of the loading popup.

**`CMSSelectLevel`** (layout `cmsselectlevel`, class `SMSSelectLevel`, 6 gadgets: scroll list `sclScrollList`, two arrow image buttons; 14 functions, 7 named). The **list of single player levels**.

- `PostCreate` (118 bytes) fills the scroll list with **level items** (profiled; the item class carries one string, the **world path**: the declared member `msli_strWorldPath`).
- `OnSetCurrent` (232 bytes, read): focuses the list, takes the profile's **current level name** and **selects the list row whose name equals it** (a loop over the row count, a string compare per row).
- `SelectLevelItem_OnSelect` (199 bytes, profiled): stores the chosen world path in the variable **`SelectedLevel`** and opens `CMSSinglePlayerSettings`.
- `ibtnArrowUp_OnClick`, `ibtnArrowDown_OnClick`: scroll the list; `LoadLevel_OnSuccess/_OnFailed`: as above.
- The same pattern serves **`CMSSelectSessionLevel`** (`menu-classes-session.md`) with the variable carried to the session settings.

**`CMSSinglePlayerSettings`** (layout `cmssingleplayersettings`, class `CMSSinglePlayerSettings`, 19 gadgets: **7 option boxes**, 6 labels, 2 images, buttons `btnSelectLevel` and `btnStartGame`; 24 named methods). The **single player game set-up**.

- **Option boxes and handlers (script, high).** The seven option boxes `opslDifficulty`, `opslEnableRocketJumping`, `opslEnableSprinting`, `opslEnemyMaxMultiplier`, `opslEnemyMultiplier`, `opslUseComboWeapons`, `opslUseLives` each have an `_OnChange` handler (7 of 7).
- `UpdateState` (1,242 bytes, profiled): sets the boxes from the **profile** (difficulty, lives, rocket jumping, sprinting, combo weapons, the two multipliers); logs `Profile %1 could not have been obtained!` when there is none. `CollectProfileOptions` (543 bytes): the reverse for the four **yes/no options**, written to the profile and the game info. `GetBOOLValue`, `SetBOOLValue`: a yes/no box as a boolean (index 0 is TRUE, below).
- `opslDifficulty_OnChange` (271 bytes) updates the picture (`SetDifficultyTexture`, 534 bytes: one picture per value); `opslEnemyMultiplier_OnChange` and `opslEnemyMaxMultiplier_OnChange` write the multiplier to the **game parameters and the profile**: the maximum-multiplier handler **reads the percentage out of the chosen option's text** (`50%` to `1000%`, `%1%%`) and the text `NONE` of the multiplier box gives 0 (read, low: the handler bodies were partly shared stubs). `lblAsterixMulti`, `lblAsterixMaxMulti`, `lblAsterixWarning`, `lblMultiplierWarning` are labels shown when a multiplier differs from its default (profiled, name only for the condition).
- `btnSelectLevel_OnClick` collects the options and opens `CMSSelectLevel`; **`btnStartGame_OnClick` (read)** collects the options, takes the screen's `SelectedLevel` (the default level when empty) and **starts the game on that level**, then closes the screen. `OnCommandPressed` (59 bytes) calls `CollectProfileOptions` first, so every command stores the options.
- The in-game variant of the layout is reached through the gadget `lnkSinglePlayerSettingsInGame` (profiled).

**`CMSLoadGame`** (layout `cmsloadgame`, class `CMSLoadGame`, 12 gadgets: nine labels, the list `lstLoadGameList` (kind `LoadGameList`), image, button bar; 5 named methods). The **load list**.

- `OnSetCurrent` focuses the list. **`LoadLevelItem_OnSelect`** (214 bytes) logs `<name> : <path>` and loads the selected save game (profiled).
- `Render` (712 bytes) draws the **details of the highlighted save** into labels: `lblScore`, `lblKills`, `lblSecrets`, `lblCurrentPlaytime`, `lblTotalPlaytime`, `lblHighScore`, `lblCheated` (through `OutputValue`, 456 bytes, in the fonts `SeriousVerdana` and `SeriousZurichSmall`); `Step` writes the current date and time into `lblCurrentTime` with the format `%a, %c` (weekday and locale date and time).
- **List row (declared members of `CListRowData`, 11).** `li_strSaveName`, `li_strTimestamp`, `li_strTotalPlaytime`, `li_strStatPlaytime`, `li_strStatScore`, `li_strStatSecrets`, `li_strStatKills`, `li_strStatLives`, `li_bCheating`, `li_strSaveFilePath`, `li_strHighScore` (the save list used by `CMSSaveGame` has the first ten). These are the **text fields of one save** shown in the details, matching the contents of the save's `.nfo` (`save-game.md`).

**`CMSSaveGame`** (layout `cmssavegame`, class `CMSSaveGame`, 5 gadgets: list `lstSaveGame`, `lblCurrentTime`; 8 named methods). The **save list** with a *new save* row.

- **File name rule (read, medium).** `CreateSaveGameFileName` lists the files `*.nfo` of the profile's save folder, **sorts** them and takes the **last** name. If that name is not `QuickSave`, it reads the number out of `Save_<n>` and uses **n + 1**; the new name is `<folder>Save_<n+1>`. With no file at all the code has the default `Save_0000` (the digits are printed with a width, probably 4; low). `QuickSave` is the reserved name of the quick save slot and never becomes the base of the counter.
- **Flow (read).** Selecting the *new save* row (`SaveLevelItem_OnNewSaveGame`) opens a text popup (`CPopupMenuEnterText`, "Enter name", message `NewSaveGame`, preset text by `SetEditText`); `NewSaveGame_OnAccept` takes the variable `EnteredText`, builds the file name and saves. Selecting an **existing** row (`SaveLevelItem_OnSelect`) opens a yes/no popup "Overwrite?" (message `Overwrite`, focus `btnYes`); `Overwrite_OnYes` opens the text popup with the old name; **`Overwrite_OnAccept`** saves over the chosen item (error `Could not overwrite! Save Game list could not be obtained.` when the list is gone). `Step` shows the clock.

**`CMSRecordDemo`** (layout `cmsrecorddemo`, class `CMSRecordDemo`, 6 gadgets: `lstRecordDemo` and a save list; 7 named methods) is the **same screen for demos**: `RecordDemoItem_OnNewRecordDemo`, `NewRecordDemo_OnAccept`, `RecordDemoItem_OnSelect` (overwrite popup), `Overwrite_OnYes`, `Overwrite_OnAccept`, `OnSetCurrent`, `Step`. Its list row has three fields (`li_strDemoName`, `li_strTimestamp`, `li_strDemoFilePath`). Opened by the in-game link `lnkRecordDemo` (PC only).

**`CMSPlayDemo`** (layout `cmsplaydemo`, class `CMSPlayDemo`, 6 gadgets: `lstPlayDemoList`; 3 named methods): `OnSetCurrent` focuses the list, **`PlayDemoItem_OnSelect`** (220 bytes) logs `Starting demo playback on %1 : %2` and starts the demo, `Step` the clock. Opened by `lnkPlayDemos` of the main menu (PC only). Same row layout as the record list (`li_strDemoName`, `li_strTimestamp`, `li_strDemoFilePath`).

**`CMSStatistics`** (layout `cmsstatistics`, class `CMSStatistics`, 31 gadgets: 28 labels; 7 named methods). The **statistics screen** shown at the **end of a level** and from the in-game menu.

- `OnSetCurrent` calls `DisplayStaticStats` (894 bytes: difficulty, **kills `%1/%2`**, **secrets `%1/%2`**, score, playtime, lives and the lives caption **only when the game uses lives**, `samIsUsingLives`), `InitDynamicCounters` (1,008 bytes: creates the animated **bonus counters** for difficulty, kills, lives, secrets and time and the `lblCheated` flag) and `InitSoundChannels`. The variable `LevelFinished` tells the two uses apart (profiled).
- **`Step` (read, medium to low).** Every frame: **bonus total = time bonus + kills bonus + secrets bonus + difficulty bonus (+ lives bonus when lives are used)**, written to `lblBonusTotal`; **total score = bonus total + the base score**, written to `lblTotalScore`; then every counter in a list of counters is advanced by its increment, clamped at its target, and the counting sound is started on the first advance; when a counter reaches its target its label shows the final value. The values of the bonuses are **not computed here**: they are filled in by the game info and are the same numbers the level end uses (`inventory.md`, `players-session-classes.md`).
- `OnCommandPressed` (80 bytes) skips the animation or leaves; `OnUnsetCurrent` stops the sounds.

**`CMSMultiplayerCharacter`** (layout `cmsmultiplayercharacter`, class `SMultiplayCharacterMenuScreen`, 6 gadgets: scroll list; 7 named methods) and **`CMSMultiCharDesc`** (layout `cmsmultichardesc`, 7 gadgets: a render-model gadget and a scrolling description; 4 named methods). The **choice of the multiplayer player model**, reached from the profile settings (`btnMultiPlayerCharacters`).

- `CMSMultiplayerCharacter::PostCreate` (212 bytes) fills the scroll list with **character items** (declared members of the item class: `msmci_strModelPath`, `msmci_strThumbnailPath`, `msmci_strCharacterName`, `msmci_strDescription`); `OnSetCurrent` selects the profile's current character; **`MultiCharItem_OnSelect`** (276 bytes) stores the choice in the variable `SelectedCharacter` and opens `CMSMultiCharDesc`; `sclScrollList_OnTargetHit` handles the list's hit target; `OnCommandPressed`, `ibtnArrowUp/Down_OnClick` scroll.
- `CMSMultiCharDesc::OnSetCurrent` (345 bytes) reads `SelectedCharacter` and fills `lblCaption` and the scrolling description `stDescription`; **`Render` (4,065 bytes, the largest function of the menu family) draws the animated character model** into the render-model gadget (71 internal calls: model, animation, camera; profiled).
- Which characters exist (a table in the game parameters or a folder scan): open, T01-199.

**`CMSSelectEpisodeMovies`** (layout `cmsselectepisodemovies`, 6 gadgets; 5 named methods) and **`CMSSelectMovie`** (layout `cmsselectmovie`, 6 gadgets; 5 named methods). The **movie browser** of the extras (`lnkMovies`): the first lists the **episodes** (`PostCreate` fills it; the list item has a world path), `SelectEpisodeItem_OnSelect` stores `SelectedEpisode` and opens `CMSSelectMovie`, whose `OnSetCurrent` (291 bytes) reads that variable and lists the episode's **movies**; `SelectMovieItem_OnSelect` (102 bytes) **plays** the chosen movie (`cutscenes.md`). Arrow buttons scroll (profiled).

### 3. Yes/no option boxes

Nine screens export `GetBOOLValue(gadget id)` and `SetBOOLValue(gadget id, flag)` (`CMSCooperativeItemSettings`, `CMSCooperativeSettings`, `CMSDebugSettings`, `CMSItemSettings`, `CMSPowerupSettings`, `CMSSessionSettings`, `CMSSinglePlayerSettings`, `CMSVotingSettings`, `CMSWeaponSettings`). They take the option box by name and **read or write its index as a boolean with index 0 meaning TRUE** (read: 69 and 65 bytes, one machine body shared by the nine). The layouts agree: of the 140 two-entry option boxes **68 list `YES` first and 1 `ON` first**; only the debug screen's 67 list `No` first (script). So the first entry of a yes/no box is the "on" state everywhere except in the debug screen.

## Invariants

1. The seven option boxes of `cmssingleplayersettings` and the seven `_OnChange` handlers of `CMSSinglePlayerSettings` correspond one to one (script).
2. The difficulty option list is `TOURIST`, `EASY`, `NORMAL`, `HARD`, `SERIOUS` in every layout that has it (`cmscooperativesettings`, `cmssingleplayersettings`; `cmsoptimatch` has `ANY` before them), and the five buttons of `cmsselectdifficulty` are in the same order (script).
3. A new save game gets the number after the **last file in sorted order** (not after the largest number): file names with the same prefix and a fixed width sort like numbers (read; the width is not read).
4. Every list screen of this page opens its **next** screen by storing the chosen item in a named variable and opening the next layout by name (`SelectedLevel`, `SelectedCharacter`, `SelectedEpisode`).

## Open questions

1. Where the level, character, episode and movie lists come from, and the exact default level names: T01-200.
2. The meaning of the profile's "has a game to continue" flag, the difficulty storage in the profile (`player-rules.md`) and how the continue button resumes: T01-199.
3. The statistics bonuses (formulas, the sound channels) and the pairing with the level end: T01-200.
4. The save and demo list sources (folder names, `.nfo` fields), sorting of the lists, the zero-pad width of `Save_<n>`: T01-199 (`save-game.md`).
5. The three `Proceed` branches' exact conditions: T01-197.

## Test vectors

1. **Difficulty** (read, data): focus `btnHard`, click: `StoreSelectedDifficulty` stores **4** in the session parameters and the profile; with caller `lnkSinglePlayer` the new game starts on the level of `SelectedLevel`.
2. **Save names** (read): the folder holds `QuickSave.nfo`, `Save_0003.nfo`, `Save_0007.nfo` (sorted: `QuickSave`, `Save_0003`, `Save_0007`): the last is `Save_0007`, the new name `Save_0008`. If the folder holds only `QuickSave.nfo` the last name is `QuickSave`, no number is read and the counter stays 0: the new name is `Save_` plus 0 (`Save_0000` if the width is 4, low); an empty folder gives the code's literal `Save_0000`.
3. **Statistics** (read): kills 40 of 50, secrets 2 of 5, lives not used: the screen shows `40/50` and `2/5`, no lives line; bonus total = the four bonus counters; total score = bonus total + score.
