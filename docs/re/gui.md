# The menu system: screens, gadgets, links and the platform filter (T01-036)

- **Roadmap task:** T01-036 (feeds the UI tasks T10-xxx; focus and input rules need the oracle or a binary study)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what is stored: the 99 screen layouts, their 845 gadgets (class, name, platform mask, text, font), the link gadgets and the menu graph they form, the underlay and overlay structure, the button bars and the platform filter (all `Menus/*.rsc`, `tools/re/menus_verify.py`; members from the files' own type tables); **medium** for the roles of unnamed members (positions, sizes, colours); **low** for how the engine lays out, skins and navigates the gadgets (focus order, repeat rules, animation) and for the button-type names: those are in `EngineGui.dll` and the game module, not in the data. Scope: `Content/SeriousSam2/Menus/*.rsc` (99 files), `rsc-ctl.md` for the container.

Prerequisites: `rsc-ctl.md` (the `.rsc` container, gadget classes, fonts), `localization.md` (`CTransString` keys), `fnt.md` (fonts), `tex.md` (images), `camera-view.md`, `player-rules.md`.

## Summary

- **A menu screen is a data file.** Each of the **99 `Menus/*.rsc`** has a root `CMenuScreenLayout`: `.1` the **screen class** (an identifier such as `CMainMenu`, `CMSOptionsSelection`, `SInGameMenu`: the name that game code uses to open the screen; **the class is not unique to a file**: 6 classes have two or three layouts, for example `CMSExtras` (`CMSExtras.rsc` and `CCreditsMenuScreen.rsc`), `SInGameMenu` (`CMSInGameMenu.rsc`, `CMSCallAVote.rsc`), `SPopupMenuUnderlay` (three popup underlays), `SPopupMenuOk` and `CPopupMenuOkLarge` (with the abort-game popups) and `CMSCooperativeSettings` (with the item settings): a class names a **screen in code and its layout variants**), `.3` an **underlay** screen name and `.4` an **overlay** screen name (strings), and `.2` the list of **gadgets**. Screens are layered: **underlay** (the background: 75 use `CMSUnderlay`, 11 `CMSPopupMenuUnderlay`, 7 `CMSPopupMenuUnderlayLarge`, one the main menu underlay, one the text popup underlay, 4 none), the screen's own gadgets, and an **overlay** (73 use `CMSOverlay`, 26 none).
- **845 gadgets of 39 classes**, each with a name (`bbrButtonBar`, `lblCaption`, `lnkOptions`, `opslBloom`: the **prefix gives the kind**), a position and size, a text (key, default text, font, alignment), a colour, and a **platform mask**. The commonest: option select boxes 193, labels 187, images 106, buttons 97, button bars 78, **links 54**, image buttons 27, arrangers 26, slider boxes 15, integer select boxes 11, scroll lists 8, progress bars 5 (the rest are special: name entry, custom keys, save and load lists, server and match lists, render model, chapter list, loading background).
- **The menu flow is the link gadgets.** A **`CMenuGadgetLinkLayout`** (`.1` = the target screen's name) opens another screen. The 54 links form a small graph with **47 distinct targets**; the other screens (popups, underlays, overlays, loading, and the entry points) are opened by game code.
- **`EPlatform` is decoded here:** every gadget has a platform mask (`CMenuGadgetLayout.7`): **3 for 778 gadgets (both platforms), 1 for 55 and 2 for 12**. The mask-1 gadgets are PC-only (`lblCaptionPC`, `islMaxPlayersPC`, `btnAdvancedVideoOptions`, `lnkPlayDemos`, `lnkRecordDemo`, `lnkCustomizeButtons`, the video options `opslAnisotropy`, `opslBloom`, `opslHDRRendering`, `opslLightmapQuality`, `opslParallaxMapping`, `opslRefractions`, …); the mask-2 gadgets are Xbox-only (`btnXboxLive`, `lnkXboxLive`, `lblCaptionXbox`, `islMaxPlayersXbox`, `opslVoice`, `btnSystemLink`, `lstOptiMatchList`, …). **So bit 0 (value 1) is the PC and bit 1 (value 2) is the Xbox**; a gadget is shown on a platform whose bit is in its mask. This also explains the destruction masks (`destruction.md`): `0xFFFFFFFD` = every platform except the Xbox (the `PC` configurations) and `0xFFFFFFFE` = every platform except the PC (the `Xbox` ones).

## Evidence

`tools/re/menus_verify.py` (read-only, `SS2_DATA_DIR`; about 1 minute); the per-screen table and the link graph are `menus-tables.md` (generated):

- **Screens:** 99 of 99 are `CMenuScreenLayout` with 93 distinct screen classes; 845 gadgets; 39 gadget classes; fonts used: `SeriousZurich` 441, `SeriousVerdana` 123, `SeriousVerdanaLarge` 87, `SeriousZurich2` 30, `Zurich` 22, `ZurichStandard` 14, `SeriousZurichSmall` 14, `SeriousZurichMainMenu` 10 (the names resolve to `.fnt` files: `fnt.md`).
- **Links:** 54 link gadgets, 47 distinct targets: **40 by screen class**, 6 only by **file name** (the link names the file stem, for example `CMSCustomizeKeys`, whose screen class has another name) and **1 not found** (`SMatchListDetailMenuScreen`: the install has `SSLMatchListDetailMenuScreen`). Targets are therefore **resolved by class first, then by file stem, case-insensitively**.
- **Entry points:** 48 screens are the target of no link: `CMainMenu`, the popups (`CPopupMenuYesNo`, `…YesNoTimed`, `…EnterText`, `…OkLarge`, `…PleaseWait`, …), the underlays and overlays (`CMSUnderlay`, `CMSOverlay`, `CMSMainMenuUnderlay`), loading (`CMSLoading`), the debug and demo screens and the online and Xbox Live screens: they are opened by game code.
- **Button bars:** 78 button-bar gadgets with two lists of 4 controller buttons each (156 lists): 456 of the entries have type 0 (an empty slot) and the used types are **1 (69), 2 (76), 8 (11) and 9 (12)** (`EMenuButtonType`; the names are not in the data: by the usual menu actions, 1 and 2 are probably accept and back).

## The menu graph (main paths)

```
CMainMenu (14 gadgets; underlay CMSMainMenuUnderlay)
  lnkSinglePlayer   -> SMSProfileSelection        lnkPlayDemos   -> CMSPlayDemo
  lnkOptions        -> CMSOptionsSelection        lnkProfileSettings -> SMSProfileSelection
  lnkSelectMod      -> CMSModSelection            lnkExtras      -> CMSExtras
CMSOptionsSelection -> CMSVideoOptions, CMSAudioOptions, CMSNetworkOptions, CMSGameOptions
CMSVideoOptions     (6 option boxes, 1 slider, 4 buttons; `btnAdvancedVideoOptions` is a button, opened by code, not a link: CMSAdvancedVideoOptions has 30 gadgets, 19 option boxes, 3 sliders)
SSingleplayerMenuScreen -> CMSLoadGame, SMSSelectDifficulty, SMSSelectLevel
CMSessionSettings   -> CMSCooperativeSettings, CMSCooperativeItemSettings, CMSDeathmatchSettings, CMSVotingSettings, SMSSelectSessionLevel
SInGameMenu (17 gadgets; popup underlay) -> SProfileOptionsMenuScreen, CMSOptionsSelection, CMSSaveGame, CMSLoadGame, CMSPlayerList (PC),
                    CMSOnlinePlayerList and CMSXboxLive (Xbox), CMSCallAVote, CMSRecordDemo
SProfileOptionsMenuScreen -> CMSCustomizeKeys, CMSAdvancedOptions, SMSChangeName, SMSControllerOptions
CMSExtras           -> CMSAchievements, CMSSelectEpisodeMovies (CMSExtras.rsc); CMSSelectMovie (the credits layout of the same class)
```

- **Settings screens are data too:** the option select boxes hold the **list of options** (a `CStaticStackArray` of 2 to 20 entries) and an index; the PC video screens carry at least 19 named quality settings (anisotropy, bloom, crumbs, dynamic shadows, filtering, geometry LOD, HDR rendering, LOD bias, lens flare quality, lightmap quality and size, 1.1 shaders only, parallax mapping, particle amount, reflection quality, refractions, …) that correspond to engine settings (T01-033); the audio screen has 4 sliders and 5 option boxes; the game options 9 option boxes.
- **The key bindings screen** (`CMSCustomizeKeys`) contains a **custom keys gadget** (`CMenuGadgetCustomKeysLayout`, 2 in the data); the **default key assignments are not in the layouts** (they are in code or the profile: T01-033).

## Reader invariants

1. Identify a screen by its **class name** (`CMenuScreenLayout.1`) **and file**: a class can have several layouts (6 do); resolve a link target by file stem first when it equals one, else by class, case-insensitive, choosing the layout by the code's rule (open question); an unresolved target (the one case) is an error to log, not a crash.
2. Draw in the order **underlay, screen gadgets, overlay**; the underlay and overlay names are screens too (`CMSUnderlay`, `CMSOverlay`).
3. **Filter gadgets by the platform mask**: a PC build shows gadgets whose mask has bit 0 (values 1 and 3) and hides those with value 2; this is how one layout serves both platforms.
4. Take text from `CTransString` (key, default text) and fonts by name; positions and sizes are in each gadget (a virtual canvas: `T10-017`).
5. Menu flow beyond the links (opening popups, returning to the previous screen, the in-game menu, the loading screen) is **code**; implement it from this graph plus the behaviour observed in the original.

## Test vector

`Menus/CMainMenu.rsc`: screen class `CMainMenu`, underlay `CMSMainMenuUnderlay`, overlay `CMSOverlay`, 14 gadgets, six links as in the graph above. `Menus/CMSExtras.rsc`: class `CMSExtras`, underlay `CMSUnderlay`, overlay `CMSOverlay`, 8 gadgets (`imgSam2Logo` image, `bbrButtonBar`, `lblCaption`, `lnkAchievements` → `CMSAchievements`, `btnSpecialThanks`, `btnCredits`, `lnkMovies` → `CMSSelectEpisodeMovies`, `arrArranger`), all with platform mask 3. `Menus/CMSAdvancedVideoOptions.rsc`: 30 gadgets (19 option boxes), most with mask 1.

## Open questions

- **Runtime behaviour** (task T01-078): the layout of gadgets (anchors, the arranger), focus order and navigation by keyboard, mouse and gamepad, repeat and acceleration of sliders, animation and transitions, sounds of the menu, the skin (colours and images of focused and pressed states), the meaning of `EMenuButtonType` values, the modal behaviour of popups, and the default key bindings: oracle capture (T01-038) or a study of `EngineGui.dll` and the game module.
- The unnamed members of `CMenuGadgetLayout` (`.2`, `.3` positions and sizes, `.5` colour, `.6` a boolean) are treated as position, size, colour and visibility by their types.
- The in-game HUD elements (`SHudElementParams`, `rsc-ctl.md`) are a separate system from these menus.
