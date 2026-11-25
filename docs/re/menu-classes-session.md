# Menu classes: multiplayer, session settings, votes and match lists (T01-112, page 4 of 6)

- **Roadmap task:** T01-112 (overview: [`menu-classes.md`](menu-classes.md); this page covers the 22 classes of the "session" group)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, layouts, option lists and ranges, and for the value tables that equal the option lists or an earlier spec's enumeration (game types 2 and 4, difficulties 1 to 5, the percentage tables: script, `players-session-classes.md`); **medium** for rules marked "read" (one decompiled reading, here backed by the declared member order of the session parameters); **low** for "(profiled)", "(name)" and the Open questions. Nothing was run.

Prerequisites: [`menu-classes.md`](menu-classes.md), [`menu-classes-options.md`](menu-classes-options.md) (the connection screen), `players-session-classes.md` (`EGameType`, `GD_`, the game info), `replication.md` (section 126: the session parameters `sgp_`), `game-modes.md`, `console-config.md` (`net_bClientsMayPause`).

## Summary

The session group is the **multiplayer front end**: the entry screens (`CMSMultiplayer`, `CMSGamespy`), the server and match lists, the **session settings** (a hub with five sub-screens: cooperative, cooperative items, deathmatch, items, powerups, weapons, voting), the level choice for a session and the in-game **vote** screens. 22 classes, **304 exported functions** (154 standard + 150 named), 18 with named methods.

The key finding is that **the settings screens do not hold their own state**: every gadget reads from and writes to **the session parameters of the project instance** (the `sgp_` members: game type, difficulty, friendly fire, enemy strength, lives, maximum players, private slots, items that stay, frag and time limits, sprinting, the enemy multipliers, rocket jumping, combo weapons), through small `_OnChange` handlers and one `UpdateState` per screen that copies the parameters back to the gadgets. Leaving the session settings with the **back command resets those parameters to a freshly constructed default set** (read). The values the player chose in the single player and cooperative screens are kept in the **profile** and copied into the session parameters when a session level is picked.

## Evidence

The export names and the layouts (`tools/re/menu_classes_verify.py` sections 1 to 5); the profiles of all 150 named methods; the decompiled bodies of `CMSCooperativeSettings` (the `_OnChange` tables), `CMSSessionSettings` (state, command, game type, players), `CMSVotingSettings`, `CMSSelectSessionLevel`, `CMSCallAVote`, `CMSPlayerList`, `CMSDeathmatchSettings`. **Offset check:** the 12 session parameters the code writes are at positions that agree with the **declared member order** of the 34-member `sgp_` table (`replication.md`), six independent agreements (game type before difficulty, maximum players and private slots, frag and time limit, the enemy multiplier and its maximum, the lives and sprinting members); this is how the member names below were attached to the writes (**medium**). **Not checked:** the host side that consumes the parameters (`players-session-classes.md`), the lobby and match-making protocol (`replication.md`), the server browser behind the list gadgets, the GameSpy and Xbox network services.

## Layout / rules

### 1. The flow

```
CMainMenu --btnMultiplayer (PC)--> [connect] --> CMSProfileSelection (caller btnInternet) --> CMSGamespy
CMSMultiplayer --btnInternet--> [connect, update check] --> CMSProfileSelection ... ; --btnLocalNetwork--> CMSProfileSelection (caller btnLocalNetwork) --> CMSServerList
CMSGamespy --lnkQuickPlay--> CMSQuickMatch ; --lnkFindMatch--> CMSOptiMatch --> CMSOptiMatchList (--> CMSOptiMatchDetails)
           --lnkCreateAMatch--> CMSSessionSettings --lnkSelectLevel--> CMSSelectSessionLevel --item--> (host the session)
CMSSessionSettings --lnkCooperativeSettings / lnkCooperativeItemSettings / lnkVotingSettings / lnkDeathmatchSettings
CMSDeathmatchSettings --lnkItemSettings / lnkPowerupSettings / lnkWeaponSettings
```

The links are data (`menus-tables.md`); the caller-gadget decisions are code (`menu-classes.md` section 5).

### 2. Entry and lists

- **`CMSMultiplayer`** (layout `cmsmultiplayer`, 5 gadgets: `btnLocalNetwork`, `btnInternet` PC only; 16 functions, 9 named). The **PC multiplayer entry**. `btnInternet_OnClick` starts the online connection behind a PleaseWait popup (message `GamespyConnect`, "Connecting"); `GamespyConnect_OnSuccess` then checks for an update: `UpdateCheck_OnSuccess` (290 bytes) asks **"New update available (%1). Install?"** (message `InstallUpdate`, `_OnYes` starts the install, `_OnNo` and `UpdateCheck_OnFailed` and no update go on to the profile selection with caller `btnInternet`); `GamespyConnect_OnFailed` (390 bytes) shows an OK popup with one of `Failed to connect.`, `GameSpy service is not available at the moment. Please try again later.` or `GameSpy service is not available.` (profiled). `btnLocalNetwork_OnClick` opens the profile selection with caller `btnLocalNetwork`. `PostCreate` focuses `btnLocalNetwork`. (The main menu's own `btnMultiplayer` shares the body of `btnInternet_OnClick`.)
- **`CMSGamespy`** (layout `cmsgamespy`, 10 gadgets: links `lnkQuickPlay`, `lnkFindMatch`, `lnkCreateAMatch`, button `btnSpecifyServer`, the connection type box and the two bandwidth boxes; 16 functions, 9 named). The Internet play hub: the **connection type** with the same presets, handlers and sync functions as `CMSNetworkOptions` (`menu-classes-options.md` section 6); `OnSetCurrent` focuses `lnkQuickPlay`; `btnSpecifyServer_OnClick` opens the text popup "Specify server address" (message `SpecifyServer`); `SpecifyServer_OnAccept` (253 bytes) takes `EnteredText` and joins that address (profiled).
- **`CMSServerList`** (layout `cmsserverlist`, class `SMSServerList`, 4 gadgets: the server list `lstServerList`; 9 functions, 2 named). The LAN server list: `OnCommandPressed` (238 bytes): **command 21** (the secondary action) opens the text popup "Specify server address" and **the create command** opens `CMSSessionSettings` (profiled); `SpecifyServer_OnAccept` (253 bytes) connects to the typed address.
- **`CMSQuickMatch`** (layout `cmsquickmatch`, class `SMSQuickMatch`, 7 gadgets: a quick-match gadget and labels; 11 functions, 4 named): `PostCreate` (23 bytes) starts the search; `OnCommandPressed` (49 bytes) opens `CMSSessionSettings` (create a game instead); `ConnectingToHost_OnSuccess` (144 bytes) and `_OnFailed` (OK popup "This game session is no longer available.") answer the connection popup (profiled).
- **`CMSOptiMatch`** (layout `cmsoptimatch`, 10 gadgets: six filter boxes; 9 functions, 2 named): the **match filter**. Boxes (data): `opslGameType` (`ANY`, `COOPERATIVE`, `DEATHMATCH`), `opslDifficulty` (`ANY` then the five difficulty names), `opslFriendlyFire` (`ANY`, `YES`, `NO`), `opslNumPlayers` (`ANY`, `1` to `4`), `opslVoice` (`ANY`, `YES`, `NO`), `opslSortBy` (`LEVEL NAME`, `NUM OF PLAYERS`, `CONNECTION QUALITY`, `HOST NAME`). `OnCommandPressed` (999 bytes) builds the **filter** from the boxes (the yes/no/any boxes through `GetBOOLValueFromGadget`, which maps the texts `ETRSOptiMatch.OptionYes` and `.OptionNo`; the game type through the engine's game-type description) and opens `CMSOptiMatchList` (profiled; the filter's format is not read).
- **`CMSOptiMatchList`** (layout `cmsoptimatchlist`, 5 gadgets: two list gadgets, `lstOptiMatchList` and `lstOptiMatchListPC`; 8 functions, 1 named) shows the matches; `OnCommandPressed` (76 bytes) opens `CMSSessionSettings` (create). **`CMSOptiMatchDetails`** (layout `cmsoptimatchdetails`, class `CMSOptimatchDetails`, 5 gadgets; 10 functions, 3 named): the details of one match; `OnCommandPressed` (49) opens the session settings; `ConnectingToHost_OnSuccess` / `_OnFailed` as above. The declared members of the details gadget are the **server and client mod names and versions** (`omd_strServerMod`, `omd_strServerModVersion`, `omd_strClientMod`, `omd_strClientModVersion`): a mismatch is shown to the player.
- **The four list classes with no method:** `CCreateMatchMenuScreen` (layout `ccreatematchmenuscreen`, **0 gadgets**), `CMatchListDetailMenuScreen` (no layout; the data's `cmatchlistmenuscreen` links to a screen of this name), `CSLMatchListMenuScreen` (no layout) and `CSLMatchListDetailMenuScreen` (layout `cslmatchlistdetailmenuscreen`, class `SSLMatchListDetailMenuScreen`, 9 gadgets: five images, three labels, a button bar): the **system link** (LAN) match list and detail screens: the layouts exist (the list in the data is `cmatchlistmenuscreen`, class `SMatchListMenuScreen`) but the classes are empty (name evidence, low: T01-205).

### 3. `CMSSessionSettings` (layout `cmssessionsettings`, class `CMSessionSettings`, 15 gadgets; 20 functions, 13 named)

**Layout (data).** `opslGameType` (`COOPERATIVE`, `DEATHMATCH`; default `DEATHMATCH`), `islMaxPlayersPC` (1 to 16, default 2), `islMaxPlayersXbox` (1 to 4, default 4), `islPrivateSlots` (0 to 4, default 0), `opslInfiniteAmmo` and `opslClientsMayPause` (`YES`/`NO`, default `NO`), links `lnkVotingSettings`, `lnkCooperativeSettings`, `lnkCooperativeItemSettings`, `lnkDeathmatchSettings`, `lnkSelectLevel`.

| Method | Purpose | Basis |
|---|---|---|
| `UpdateState` (598 bytes) | copies the **session parameters to the gadgets**: `opslGameType` = coop (index 0) for game type 2 and deathmatch (index 1) for game type 4 (any other value shows coop); **enables `lnkCooperativeSettings` and `lnkCooperativeItemSettings` for coop and `lnkDeathmatchSettings` for deathmatch** (the other kind is greyed out); both max-players boxes show `sgp_ctMaxPlayers`, `islPrivateSlots` shows `sgp_ctPrivateSlots`, `opslInfiniteAmmo` shows `sgp_bInfiniteAmmo` (index 0 = YES = TRUE) and `opslClientsMayPause` the console variable **`net_bClientsMayPause`** | read, medium |
| `opslGameType_OnChange` (276 bytes) | writes **`sgp_gtGameType`**: option 0 gives `GT_COOPERATIVE` (2), option 1 `GT_DEATHMATCH` (4) (`players-session-classes.md`: 2 and 4; any other option gives 2), then enables and disables the same three links as `UpdateState` and focuses a gadget | read |
| `islMaxPlayersPC_OnChange`, `islMaxPlayersXbox_OnChange` (219 bytes each) | write **`sgp_ctMaxPlayers`** and **limit `sgp_ctPrivateSlots` to the new maximum** (the smaller of the two is stored and shown in `islPrivateSlots`) | read |
| `islPrivateSlots_OnChange` | the same pairing from the other side (writes `sgp_ctPrivateSlots`, bounded by the maximum) | read, low |
| `opslInfiniteAmmo_OnChange`, `opslClientsMayPause_OnChange` | write `sgp_bInfiniteAmmo` and `net_bClientsMayPause` (index 0 is TRUE) | read in part |
| `GetBOOLValue`, `SetBOOLValue` | the yes/no helpers (index 0 = TRUE; `menu-classes-play.md` section 3) | read |
| `OnSetCurrent` | calls `UpdateState`; hides the connection-type buttons and logo that do not apply (`btnInternet`, `btnLocalNetwork`, `btnSystemLink`, `imgGameSpyLogo` appear in the strings: the **same four names** are used by every settings sub-screen's `OnSetCurrent`, below); focuses `islPrivateSlots` | read in part |
| `PostCreate` | `UpdateState`, focuses `opslGameType` through the button bar | profiled |
| `OnCommandPressed` (376 bytes) | **command 12 (back): resets the session parameters** (the embedded parameter record of the project instance is replaced by a **newly constructed default `CSamProjectParams`**); **command 21 (online only): opens the text popup "Session name"** (message `SessionName`, preset text from the code's default) | read |
| `SessionName_OnAccept` | takes `EnteredText` as the session name, or `Unnamed session` when empty (profiled) | profiled |

**Sub-screen pattern (`CMSCooperativeSettings`, `CMSCooperativeItemSettings`, `CMSDeathmatchSettings`, `CMSItemSettings`, `CMSPowerupSettings`, `CMSWeaponSettings`, `CMSVotingSettings`).** Each has `PostCreate` (54 bytes: `UpdateState` and a focus), `Step` (16 bytes: `UpdateState` every frame, except `CMSPowerupSettings`), `UpdateState` (copy session parameters or item rules to the boxes), `OnSetCurrent` (the same 127-byte function in six classes: hides `btnLocalNetwork`, `btnSystemLink` and `imgGameSpyLogo`, which belong to the connection screens that share the layout's background), and one `_OnChange` per box that writes the matching parameter; plus the helpers `GetBOOLValue` and `SetBOOLValue` for the yes/no boxes. The `Step` call to `UpdateState` means **the screens show the live values** (so a vote or another source changing a rule is reflected at once).

### 4. The value tables

All verified against the layouts (script section 5) and, where marked, against `players-session-classes.md`.

| Screen and box | Option index to stored value | Stored member | Basis |
|---|---|---|---|
| `CMSCooperativeSettings.opslDifficulty` (also `CMSSinglePlayerSettings`) | 0, 1, 2, 3, 4 (`TOURIST`, `EASY`, `NORMAL`, `HARD`, `SERIOUS`) to difficulty 1, 2, 3, 4, 5 (`GD_TOURIST` to `GD_SERIOUS`: **two sources agree**; the option 2 and any option above 4 give 3) | `sgp_gdGameDifficulty` | read, data, `players-session-classes.md`: **high** |
| `opslEnemyMaxMultiplier` (18 options) | 50, 75, 100, 125, 150, 175, 200, 225, 250, 275, 300, 400, 500, 600, 700, 800, 900, 1000 (percent; any other index 100); default option 2 = 100 | `sgp_iEnemyMaxMultiplier` (and the profile) | read, data: **high** |
| `opslEnemyMultiplier` (11 options `NONE`, `2X` ... `11X`) | 0, 2, 3, ..., 11 | `sgp_iEnemyMultiplier` | read, data |
| `opslEnemyMultiplierPerPlayer` (10 options) | 0, 25, 50, 100, 150, 200, 250, 300, 400, 500 (percent: `1X PER 4 PLAYERS` = 25 ... `5X PER PLAYER` = 500) | `sgp_iEnemyMultiplierPerPlayer` | read, data (texts parse to the same list) |
| `opslExtraEnemyStrength`, `opslEnemyStrengthPerPlayer` (9 options each) | the number before `%` (`NONE` = 0): 0, 10, 25, 50, 75, 100, 150, 200, 400 | `sgp_iExtraEnemyStrength`, `sgp_iEnemyStrengthPerPlayer` | read, data (the damage formula of `puppet-entity-classes.md` multiplies by 0.01 times them: percent) |
| `islNumLives` (1 to 10, default 3), `opslUseLives` | the number; TRUE for option 0 (also copied to the profile) | `sgp_ctMaxLives`, `sgp_bUseLives` | read |
| `opslFriendlyFire`, `opslRespawnInPlace`, `opslEnableRocketJumping`, `opslUseComboWeapons`, `opslEnableSprinting` (cooperative) | TRUE for option 0 (`YES`) | `sgp_bFriendlyFire`, `sgp_bRespawnInPlaceInCoop`, `sgp_bEnableRocketJumping`, `sgp_bUseComboWeapons`, `sgp_bEnableSprinting` | read in part |
| `CMSCooperativeItemSettings`: `opslWeaponsStay`, `opslAmmoStays`, `opslHealthArmorStays`, `opslDisableSeriousBombs` | TRUE for option 0 (the layout defaults: `NO` for the first three, `YES` for the bombs) | `sgp_bWeaponStaysInCoop`, `sgp_bAmmoStaysInCoop`, `sgp_bHealthAndArmorStayInCoop`, `sgp_bDisableSeriousBombs` | name, data (the member names match one to one) |
| `CMSDeathmatchSettings`: `islFragLimit` (1 to 200, default 20), `islTimeLimit` (1 to 200, default 10) | the number (the time limit's unit is not read: minutes by the rules of the genre, low) | `sgp_ctFragsLimit`, `sgp_ctTimeLimit` | read (`UpdateState` reads both from the parameters) |
| `opslGameType` (session) | 0 to `GT_COOPERATIVE` 2, 1 to `GT_DEATHMATCH` 4 | `sgp_gtGameType` | read, `players-session-classes.md` |
| `islMaxPlayersPC` (1 to 16, default 2), `islPrivateSlots` (0 to 4) | the number | `sgp_ctMaxPlayers`, `sgp_ctPrivateSlots` | read |

The coop layout's defaults (script): difficulty `NORMAL`, lives on and 3, friendly fire `NO`, respawn in place `YES`, rocket jumping `YES`, combo weapons `NO`, sprinting `NO`, max multiplier 100%. `CMSItemSettings` (11 boxes: health `XS`, `S`, `M`, `L`, `XL` and armor `XS`, `S`, `M`, `L`, `XL`, `XXL`), `CMSPowerupSettings` (4: serious speed, serious damage, invisibility, invulnerability) and `CMSWeaponSettings` (14 weapons) are **per-item allow lists**: each `_OnChange` (110 bytes) **adds or removes the item's name from the session's disabled-item list** (the strings are the item names `HealthXS` ... `ArmorXXL`, `InvisibilityPowerUp`, `InvulnerabilityPowerUp`, `SeriousDamage`, `SpeedPowerUp`, and for weapons `AutoShotgun`, `BeamGun`, `Cannon`, `CircularSaw`, `DoubleShotgun`, `GrenadeLauncher`, `Klodovik`, `MiniGun`, `PlasmaRifle`, `RocketLauncher`, `Sniper`, `ZapGun`; for `opslDoubleColts` and `opslDoubleUzis` the handlers carry no name string: shared code, low). All boxes default to `NO` in the layout (index 1). Which list holds the names (the session's item filters) is **not read**: T01-201.

### 5. Votes, level choice and player list

- **`CMSVotingSettings`** (layout `cmsvotingsettings`, 8 gadgets: four boxes; 17 functions, 10 named): `opslNextMapAllowed`, `opslRestartMapAllowed`, `opslVoteKickAllowed`, `opslTimeBanAllowed`. The code names the **four vote kinds** with strings **`nextmap`, `restart`, `kick`, `timeban`**; `UpdateState` reads, for each kind, whether the vote is **allowed** (a function on the vote list) and shows YES or NO; each `_OnChange` (196 bytes) **allows or disallows** that vote kind by name (read for `nextmap`, profiled for the others). The same four names equal the four buttons of `CMSCallAVote` (data: `btnNextMap`, `btnRestartMap`, `btnKickPlayer`, `btnTimeBanPlayer`) and the console commands in `console-config.md` (`samNextMap`, `samRestartMap`, `samKickPlayer`, `samTimeBan...`).
- **`CMSCallAVote`** (layout `cmscallavote`, class `SInGameMenu`, 6 gadgets: four buttons, popup underlay; 13 functions, 6 named): `btnNextMap_OnClick` and `btnRestartMap_OnClick` (10 bytes each) **call the vote** and close the menu; `btnKickPlayer_OnClick` and `btnTimeBanPlayer_OnClick` open `CMSPlayerList` and put it in **selection mode**; `PlayerList_PlayerSelected(player)` (98 bytes) calls the kick vote or the time-ban vote for the chosen player (depending on which button was pressed) and closes. `PostCreate` (54 bytes) disables `btnRestartMap` in some state (profiled). Opened by the in-game link `lnkCallAVote` (PC only).
- **`CMSPlayerList`** (layout `cmsplayerlist`, 4 gadgets: `lstPlayerList`; 11 functions, 4 named): the **player list**. In selection mode `PlayerList_OnSelect` (210 bytes) closes the screen and raises the event `PlayerSelected` with the player index on the screen below (this is how `CMSCallAVote` gets it); otherwise `OnSetCurrent` puts the entry **"kick player"** into the button bar and `KickPlayer_OnYes` kicks the highlighted player (`KickPlayer_OnNo` empty). Opened from the in-game menu (`lnkPlayerListPC`; the Xbox build uses `CMSOnlinePlayerList`, `menu-classes-online.md`).
- **`CMSSelectSessionLevel`** (layout `cmsselectsessionlevel`, class `SMSSelectSessionLevel`, 6 gadgets; 12 functions, 5 named): the multiplayer **level list** (the same pattern as `CMSSelectLevel`, `menu-classes-play.md`). **`SelectLevelItem_OnSelect` (279 bytes, read):** if the **network cable is not connected** it sends the message `Message.Network.CableDisconnected` to the player and stays; otherwise it closes the screen, **copies the profile's six game options** (enemy multiplier, maximum multiplier, use lives, rocket jumping, combo weapons, sprinting) **into the session parameters**, and **starts the session on the chosen level**. `OnSetCurrent` hides the connection logos as above and focuses the list.

## Invariants

1. Every session setting of section 4 has exactly one box whose `_OnChange` writes it; a box's option count equals the code's table size (18, 11, 10, 9, 5, 2: script).
2. `GT_COOPERATIVE` is 2 and `GT_DEATHMATCH` is 4 (two sources: code and `players-session-classes.md`); the difficulty values are 1 to 5.
3. The maximum number of players is at least the private slot count after any change of either (read: the code takes the smaller value).
4. The cooperative and deathmatch sub-screens are **mutually exclusive links** of the session settings (the other kind is disabled): a deathmatch session has no cooperative settings screen (read).
5. The session parameters are reset to defaults when the session settings are left with the back command (read; one function).

## Open questions

1. Which list the item, powerup and weapon switches write (the disabled-item names), the format of the OptiMatch filter and the lobby protocol: T01-201.
2. The profile's six saved options (their storage and the file): T01-199.
3. The units and effect of `sgp_ctTimeLimit`, `net_bClientsMayPause` and the private slots in a running session: T01-201 (`replication.md`).
4. The empty system link classes and `CCreateMatchMenuScreen`: T01-205.
5. The vote functions' effect on the server (admin rights, timers): T01-201.

## Test vectors

1. **Game type** (read, two sources): the session settings with `opslGameType` at index 1 (default) store 4; choosing option 0 stores 2 and re-enables the two cooperative links and disables `lnkDeathmatchSettings`.
2. **Max multiplier** (read, data): option index 11 stores 400; index 17 stores 1,000; the default option 2 stores 100.
3. **Per-player enemy count** (read, data): `opslEnemyMultiplierPerPlayer` option 1 (`1X PER 4 PLAYERS`) stores 25; option 9 stores 500; option 0 (`NONE`) stores 0.
4. **Maximum players** (read): with private slots 3, setting the maximum players to 2 stores 2 in both (the private slot count follows).
5. **Back out** (read): leaving the session settings with command 12 replaces the session parameters by a default-constructed set.
