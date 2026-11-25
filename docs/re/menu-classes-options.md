# Menu classes: the options screens and the video quality presets (T01-112, page 3 of 6)

- **Roadmap task:** T01-112 (overview: [`menu-classes.md`](menu-classes.md); this page covers the 12 classes of the "options" group)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, layouts, option lists, ranges and the handler-to-gadget matches (script); **high** for the numbers of the quality, connection and multiplier tables **as read from the binary once and equal to the data's option lists where a list exists** (the lists and counts agree; the table values themselves are one reading: **medium**); **medium** for rules marked "read"; **low** for "(profiled)", "(name)" and the Open questions. Nothing was run.

Prerequisites: [`menu-classes.md`](menu-classes.md), `console-variables.md` (all 36 variables named below are listed there), `console-config.md` (the `sam_`, `cli_`, `gfx_` families), `replication.md` (the bandwidth variables).

## Summary

The options group is the **settings screens**: the hub, video (with an advanced page), audio, game, network, the player's advanced options, the controller options with the key and button customisation, and the Renovation mod's options. 12 classes, **226 exported functions** (84 standard + 142 named), 10 of them with named methods. Almost every handler follows one scheme: each gadget has an `_OnChange` (or none) and the screen copies **all gadgets to the variables when the screen closes or Apply is pressed** (`Sync...FromGadgets`), and copies the variables to the gadgets when it opens (`Sync...ToGadgets`).

## Evidence

The export names, the layouts of the base game and of the Sam2Renovation tree (`tools/re/menu_classes_verify.py`, sections 4, 5 and 8); the profiles of all 142 named methods; the decompiled bodies of `CMSVideoOptions` (quality sync, store, resolution, network), `CMSNetworkOptions` (both sync functions), `CMSCooperativeSettings`-style tables; the preset table of the video screen was **dumped** (28 variables x 5 levels, below). **Not checked:** which variable each of the other gadgets writes (video resolution and display mode paths, the advanced screen's six groups, audio, game options, controller): the variable names a function uses are not strings in the code; T01-196. Nothing was applied.

## Layout / rules

### 1. The scheme

- **Open:** `OnSetCurrent` or `PostCreate` calls `Sync...ToGadgets` (variables to gadgets) and, in the video screens, `StoreInitValues` (remembers the values at entry to detect a change and to undo it).
- **Change:** `<gadget>_OnChange` handlers exist for the gadgets that need an immediate side effect (a preview, a dependent gadget); for the others (the nine game options, six Renovation options) the handler is an empty or one-call function and the value is read at the end.
- **Close or Apply:** `OnCommandPressed` (command 12, back) with a pending change asks **"A change has been detected. Do you want to apply these settings now?"** (yes/no popup, message `msgExitScreen`; `_OnYes` applies then leaves, `_OnNo` leaves). The **Apply** button asks "Apply these settings (this might take a while)?" (message `btnApply`), then `ApplyChanges`.
- **Defaults:** `btnDefaultSettings_OnClick` asks "Restore default settings?" (message `btnDefaultSettings`, texts `ETRSVideoOptionsMenu.ResetToDefaults` or `ETRSOptionsMenu.ResetToDefaults`), `_OnYes` writes the default values to the variables and then to the gadgets. all 7 classes with such a button follow this (`CMSVideoOptions`, `CMSAdvancedVideoOptions`, `CMSAudioOptions`, `CMSGameOptions`, `CMSNetworkOptions`, `CMSAdvancedOptions`, `CMSRenovationOptions`); `CMSNetworkOptions` also has an empty `_OnNo`.

### 2. `CMSOptionsSelection` (layout `cmsoptionsselection`, 7 gadgets: four links; no method)

Links `lnkVideoOptions` (to `CMSVideoOptions`), `lnkAudioOptions`, `lnkNetworkOptions`, `lnkGameOptions`. **The Sam2Renovation tree's copy has a fifth link `lnkRenovationOptions`** (to the Renovation options screen). Opened from the main menu and the in-game menu (`lnkOptions`).

### 3. Video: `CMSVideoOptions` (layout `cmsvideooptions`, 15 gadgets) and `CMSAdvancedVideoOptions` (30 gadgets)

**Layout (data).** `cmsvideooptions`: `opslDisplayMode` (4: `FULL SCREEN`, `WINDOWED`, `BORDERLESS`, `MAXIMIZED`), `opslResolution` (20 entries in the layout, replaced at run time), `opslAntiAliasing` (4: `NONE`, `LOW`, `MEDIUM`, `HIGH`), `opslTextureSize` and `opslQuality` (6 each: `MINIMUM`, `LOW`, `MEDIUM`, `HIGH`, `MAXIMUM`, `CUSTOM`), `opslMaxFPS` (8: `NO LIMIT`, `30`, `60`, `90`, `120`, `144`, `240`, `CUSTOM`), slider `sldGamma` (default 50, step 2), buttons `btnApply`, `btnDefaultSettings`, `btnAutoDetect` and (PC) `btnAdvancedVideoOptions`.

**Methods of `CMSVideoOptions` (42 functions, 35 named).**

| Method | Purpose | Basis |
|---|---|---|
| `PostCreate`, `OnSetCurrent` | fill the resolution box (`FillResolutionsGadget`: one entry `<w>x<h>` per mode from `GetAvailableResolutions`), `StoreInitValues`, `SyncOptionsToGadgets`; the box of `opslQuality` is focused | profiled |
| `StoreInitValues` | records the **current display state**: resolution, window size, `tex_iMaxSize`, `lmp_iMaxSize`, `gfx_iAntiAliasing`, and the three display flags; computes the **display mode index** 0 (full screen), 2 (simulated full screen = borderless), 3 (maximised), 1 otherwise (windowed) | read; **equals the order of the option list** (data) |
| `SyncCvarsFromQuality(level)` | the **quality preset**: clamps the level to 0..4, copies **column `level` of the preset table** (section 3.1) into the 28 variables, then sets eight variables to fixed values | read, medium |
| `SyncCvarsToQuality` | finds the **first level whose 28 variables all equal** the current values (equal within a small tolerance) and selects it in `opslQuality`; if none matches it selects **5 = CUSTOM** | read, medium |
| `SyncOptionsToGadgets` / `SyncOptionsFromGadgets` | variables to gadgets and back for the resolution, display mode, anti-aliasing, maximum FPS, texture size, quality and gamma (the variable of each is not mapped) | profiled |
| `opslQuality_OnChange` (548 bytes) | when the player picks a preset, **sets the dependent boxes** (anti-aliasing, texture size, resolution, display mode strings) from it and runs `SyncCvarsFromQuality` | profiled |
| `opslDisplayMode_OnChange` | changing the display mode refills the resolution box (`SetResolutionToGadget`) | profiled |
| `opslTextureSize_OnChange` | one-call update | profiled |
| `sldGamma_OnChange` | applies the gamma immediately (and scrolls the arranger to keep the slider visible) | profiled |
| `btnApply_OnClick` / `_OnYes` / `_OnNo` | confirm, then `ApplyChanges` + `SyncOptionsFromGadgets` + `SyncOptionsToGadgets`; `_OnNo` resets the gadgets from the variables | read / profiled |
| `ApplyChanges` | runs the console functions **`gfxRestart`** and **`shaReloadShaders`** (the strings it uses), then `StoreInitValues` | read |
| `msgKeepSettings_OnYes` / `_OnNo` | the **keep or revert** answer after a display change: yes does nothing more; no runs `gfxRestart` and puts the old resolution back (`RestoreInitDisplayModeValues`, `SetResolutionToGadget`). Which popup raises `msgKeepSettings` is **not seen** (probably the timed yes/no, `menu-classes-popups.md`: T01-203) | read, low |
| `btnAutoDetect_OnClick` | **Auto-Detect Quality**: runs the engine's performance test (`gfxTestPerformance`), selects the preset it returns in `opslQuality`, runs the preset (`opslQuality_OnChange`, `SyncCvarsFromQuality`), `gfxRestart`, `shaReloadShaders`, and sets `sha_iRefractionMip` | read in part |
| `btnAdvancedVideoOptions_OnClick` / `_OnYes` / `_OnNo` | if there are pending changes asks to apply them first; opens `CMSAdvancedVideoOptions` | read |
| `ExecuteFunc`, `GetAvailableResolutions`, `GetResolutionIndex`, `GetResolutionFromGadget`, `SetResolutionToGadget`, `RestoreInitDisplayModeValues`, `ResetVideoOptions`, `Step` | helpers (`ExecuteFunc`: run a console function by name; `GetResolutionIndex(w, h)`: the index in the list of modes or -1) | read / name |

**`CMSAdvancedVideoOptions`** (40 functions, 33 named) is the same machinery with **six variable groups**, one `Sync<Group>ToGadgets` and `Sync<Group>FromGadgets` pair each (the prefixes are the variable families of the quality table): `Crm` (crumbs: `opslCrumbs`), `Efx` (effects: `opslBloom`, `opslLensFlareQuality`), `Gfx` (`opslHDRRendering`, `opslWaitVSyncs`, sliders `sldBrightness`, `sldContrast`, `sldGamma`), `Ren` (`opslDynamicShadows`), `Sha` (`opslReflectionQuality`, `opslRefractions`), `Tex` (`opslAnisotropy`, `opslFiltering`, `opslLODBias`, `opslLightmapQuality`, `opslLightmapSize`, `opslTextureQuality`, `opslTextureSize`). Further boxes in the layout without a group function in the name list: `opslParticleAmount`, `opslParallaxMapping`, `opslOnly11Shaders`, `opslGeometryLOD`. `OnCommandPressed` (620 bytes) asks to apply on leaving (large yes/no popup `CPopupMenuYesNoLarge`, message `msgExitScreen`) and scrolls the arranger; `OnSetCurrent` focuses the arranger and greys out `opslHDRRendering` and `opslOnly11Shaders` according to hardware (profiled, low). Only 3 `_OnChange` handlers (`opslBloom`, `opslCrumbs`, `opslDynamicShadows`) and three slider handlers exist: the rest is read at Apply. The layout is a **mixed** list of presets: `opslDynamicShadows` has `NONE`, `MINIMUM`, `LOW`, `MEDIUM`, `HIGH`, `CUSTOM`; `opslCrumbs` `NONE`, `LOW`, `OPTIMAL`, `HIGH`, `MAXIMUM`, `CUSTOM`; `opslBloom` `NONE`, `DARKER`, `OPTIMAL`, `BRIGHTER`, `GLARING`, `CUSTOM`; `opslLensFlareQuality` `NONE`, `CORONA`, `REFLECTIONS`, `GLARING`; `opslLODBias` `-3`, `-2`, `-1`, `0`, `1`, `CUSTOM`. How each option maps to variable values: T01-196.

#### 3.1 The quality preset table (28 variables x 5 levels)

The columns are the levels 0 to 4 of `opslQuality` (`MINIMUM`, `LOW`, `MEDIUM`, `HIGH`, `MAXIMUM`); the numbers are the values as read (floats written without trailing zeros). The unit of each variable is that of `console-variables.md`; the 5 levels of `tex_iMaxSize` are the integers 10 to 14 (the default of the variable is 8: the unit and whether the preset exceeds the default **are not established**).

| Variable | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| `tex_iMaxSize`, `lmp_iMaxSize` | 10 | 11 | 12 | 13 | 14 |
| `tex_iQuality` | 3 | 3 | 0 | 0 | 0 |
| `lmp_iQuality` | 3 | 0 | 0 | 0 | 0 |
| `gfx_iAntiAliasing` | 0 | 0 | 1 | 2 | 2 |
| `tex_iFiltering` | 21 | 22 | 22 | 22 | 22 |
| `tex_iAnisotropy` | 1 | 1 | 2 | 4 | 8 |
| `tex_fLODBias` | 0 | 0 | -3 | -3 | 0 |
| `ren_fLODBias` | 1.5 | 1 | 0.75 | 0.5 | 0.25 |
| `ren_fOcclusionSizeTreshold` | 0.9 | 0.7 | 0.5 | 0.3 | 0.1 |
| `ren_iDynamicShadowQuality` | 0 | 1 | 2 | 3 | 3 |
| `ren_fDynamicShadowSlanting` | 0.1 | 0.25 | 0.5 | 0.75 | 1 |
| `ren_fDynamicShadowCullSize` | 0.1 | 0.05 | 0.03 | 0.02 | 0.01 |
| `ren_fDynamicShadowFadeSize` | 0.2 | 0.1 | 0.05 | 0.03 | 0.01 |
| `ren_pixDynamicShadowSize` | 128 | 256 | 256 | 512 | 1024 |
| `efx_iLensFlareQuality` | 0 | 1 | 3 | 3 | 3 |
| `efx_fParticleBias` | 0.75 | 1 | 1.25 | 1.5 | 2 |
| `efx_fBloomBias` | 0 | 0.8 | 1 | 1.1 | 1.2 |
| `crm_bAllowRendering` | 0 | 1 | 1 | 1 | 1 |
| `crm_fDistanceBias` | 0.5 | 0.75 | 1 | 2 | 3 |
| `crm_fDensityBias` | 0.5 | 0.666 | 1 | 1.1 | 1.25 |
| `sha_bFastFlatCubeReflections` | 1 | 1 | 0 | 0 | 0 |
| `sha_bForceFlatCubeReflections` | 1 | 0 | 0 | 0 | 0 |
| `sha_bAllowRefractions` | 0 | 0 | 1 | 1 | 1 |
| `sha_iRefractionMip` | 3 | 2 | 1 | 0 | 0 |
| `sha_bNoOffsetMapping` | 1 | 1 | 0 | 0 | 0 |
| `d3d_iMaxAllowedPixelProgramVersion` | 11 | 11 | 30 | 30 | 30 |
| `d3d_bForcePixelProgramVersion2` | 0 | 0 | 0 | 1 | 1 |

Variables set to a fixed value at **every** level: `lmp_bAllowAnisotropy` 0, `mdl_bHardwareSkinning` 1, `mdl_bCacheAnimations` 1, `ren_bOcclusionCulling` 1, `ren_bEffectOcclusion` 0, `ren_bVisibilityGroups` 1, `tex_iBaseMip` 0, `lmp_iBaseMip` 0. All 36 names are listed in `console-variables.md`. Reading the table next to the option lists: the quality list has **6** entries = 5 levels + `CUSTOM` (script); `tex_iFiltering` 21 and 22 pair with the two filtering options `BILINEAR` and `TRILINEAR`, `tex_iAnisotropy` with `NONE`, `2x`, `4x`, `8x`, `16x` (the preset never uses 16x); `d3d_iMaxAllowedPixelProgramVersion` 11 and 30 are shader model codes (1.1 and 3.0, matching `opslOnly11Shaders`: name evidence, low).

### 4. Audio: `CMSAudioOptions` (layout `cmsaudiooptions`, 14 gadgets; 20 functions, 13 named)

Layout (data): sliders `sldMasterVolume`, `sldMusicVolume`, `sldEffectsVolume`, `sldMovieVolume` (each default 50, step 10), boxes `opslMenuMusic` (`ON`, `OFF`), `opslVoice` (`LOW`, `HIGH`), `opslEAX`, `opslMaxSounds` (10, 15, 20, 25, 30), `opslSoftwareEmulation` (`NONE`, `STEREO ONLY`, `LITE HRTF`, `FULL HRTF`), `btnDefaultSettings`. Handlers: **eight `_OnChange` functions of 7 bytes each**, all calling one shared function: **the audio options apply at once**, there is no Apply step (profiled). `OnSetCurrent` (70 bytes) greys out `opslSoftwareEmulation` when it does not apply (profiled); `btnDefaultSettings_OnYes` (91 bytes) writes the defaults. The handler `sldVoice_OnChange` has **no gadget** in the layout (the box is `opslVoice`).

### 5. Game: `CMSGameOptions` (layout `cmsgameoptions`, 14 gadgets; 21 functions, 14 named)

Nine option boxes: `opslNetricsaVoice`, `opslNetricsaSubtitles`, `opslSamVoice`, `opslSamSubtitles`, `opslNpcVoice`, `opslNpcSubtitles` (all `YES`/`NO`), `opslShowPuppetRadar` (6: `NO`, `RIGHT TOP`, `RIGHT MIDDLE`, `RIGHT BOTTOM`, `CENTER BOTTOM`, `LEFT MIDDLE`), `opslUseComboWeapons`, `opslEnableSprinting`. **All nine have an `_OnChange` of 5 bytes (empty)** (script and read): the values are taken at once from `SyncOptionsFromGadgets` (815 bytes) when the screen closes and written by `SyncOptionsToGadgets` (836 bytes); both use the **game info entity** (the sprinting and combo-weapon flags are per-session parameters, the subtitle and voice flags are `sam_` variables: `console-config.md`). `btnDefaultSettings_OnYes` (164 bytes) writes defaults to the game info and refreshes the gadgets.

### 6. Network: `CMSNetworkOptions` (layout `cmsnetworkoptions`, 7 gadgets; 17 functions, 10 named)

- **Layout (data).** `opslConnectionType` (5 entries: `56k MODEM`, `ISDN`, `CABLE/ADSL`, `CUSTOM`, `CUSTOM`), integer boxes `islMaxBPSUp` (2,000 to 10,000, default 7,500, step 500) and `islMaxBPSDown` (10,000 to 200,000, default 100,000, step 5,000), `btnDefaultSettings`.
- **Presets (read, medium; script: inside the ranges).** Option 0 = **3,000 up / 12,000 down**, option 1 = **5,000 / 65,536**, option 2 = **7,500 / 100,000**; option 3 = custom (the fifth entry, a second `CUSTOM`, is never produced). Upstream is the variable **`cli_iMaxOutBPS`** and downstream **`cli_iMaxBPS`** (`replication.md` table: defaults 7,500 and 100,000, i.e. option 2). All three presets lie inside the integer boxes' ranges and option 2 equals the boxes' defaults (script).
- **Sync (read).** `SyncOptionsToGadgets(useDefaults)`: with `useDefaults` false, picks the preset whose pair equals the current variables, else 3 (custom); the integer boxes show the values and are **enabled only for custom**; with defaults true selects option 2 and 7,500 / 100,000. `SyncOptionsFromGadgets`: preset 0 to 2 copies the preset pair into the screen's two members (`ms_iUpstreamBPS`, `ms_iDownstreamBPS`, the only two declared members of this screen) and shows them in the boxes; custom reads the boxes. The three `_OnChange` handlers (`opslConnectionType`, `islMaxBPSUp`, `islMaxBPSDown`) each call `SyncOptionsFromGadgets`; `OnSetCurrent` calls `SyncOptionsToGadgets`; `OnCommandPressed` (70 bytes) stores the two members into the variables on the way out. `btnDefaultSettings`: yes/no (large style `CPopupMenuYesNo`, answers `_OnYes`, `_OnNo`).
- `CMSGamespy` (`menu-classes-session.md`) has the same two members, the same three handler names and `SyncOptions...` pair: the multiplayer entry screen embeds the connection type.

### 7. Player options: `CMSAdvancedOptions`, `CMSControllerOptions`, key screens

**`CMSAdvancedOptions`** (layout `cmsadvancedoptions`, 22 gadgets; 14 functions, 7 named). The **profile's input and view options**. Boxes: `opslInvertLook`, `opslAutoCenterView`, `opslAutoAiming`, `opslAutoWeaponChange` (3: `YES`, `NO`, `ONLY IF STRONGER`), `opslViewBobbing`, `opslUseAutoJump`, `opslFlipFireButtons`, `opslMouseSmoothing`, `opslShowPuppetRadar`, `opslWeaponsList` (`HORIZONTAL`, `VERTICAL`), `opslPrefersThirdPersonView` (**`NO` first**, the one yes/no box besides the debug screen that lists the negative first), `opslCrosshairColor` (8: `WHITE`, `RED`, `GREEN`, `BLUE`, `YELLOW`, `CYAN`, `MAGENTA`, `BLACK`); sliders `sldSensitivity`, `sldDeadZone`, `sldCrosshairSize` (defaults 50, 50, 75); integer box `isRadarOffsetY` (-200 to 200, step 5); an image select box `imslCrosshairTexture`. `CollectProfileOptions` (1,892 bytes) **reads all of them into the profile** (it logs `Profile %1 could not have been obtained!` when there is none) and runs on every command (`OnCommandPressed` calls it). `opslCrosshairColor_OnChange` and `sldCrosshairSize_OnChange` set the **colour and size of the preview** `imslCrosshairTexture` (the declared members `mgl_iColor` and `mgl_fCrosshairSize` of that gadget class, `menu-gadget-members.md`). `PostCreate` and `btnDefaultSettings_OnYes` also touch a gadget `opslMouseLook` that this layout lacks. The `CProfileOptionsMenuScreen` hub opens this screen as `lnkGameOptions`.

**`CMSControllerOptions`** (layout `cmscontrolleroptions`, class `SMSControllerOptions`, 29 gadgets: 21 labels; 18 functions, 11 named). Gamepad options: `opslAxesControls` (4: `standard`, `lefty`, `beginner`, `lefty beginner`), `opslButtonsControls` (8: `custom`, `default`, `standard`, `standard A`, `alternative`, `lefty`, `standard B`, `alternative B`), `opslAutoAim`, link `lnkCustomizeButtons`. **Declared members** (the binary's only member table of a screen class): `ms_astrButtonDescLabels`, `ms_astrButtonControls` (arrays of strings), `ms_uwButtonsCount`, `ms_bButtonsActive`, `ms_bAxesActive`, `ms_eAxesPreset` (the preset enumeration). The `_OnSetFocus`/`_OnLoseFocus` pair of each preset box sets `ms_bAxesActive` or `ms_bButtonsActive`; **`Render`** draws the **button description labels** of the focused preset (profiled); `opslAxesControls_OnChange` (184 bytes) and `opslAutoAim_OnChange` (141) write the profile; `OnSetCurrent` (225 bytes) loads them.

**`CMSCustomizeKeys`**, **`CMSCustomizeButtons`** (layouts `cmscustomizekeys`, `cmscustomizebuttons`, classes `SMSCustomizeKeys`, `SMSCustomizeButtons`, 6 gadgets each: the custom keys gadget `mgCustomizeKeys` and two arrow buttons; 11 functions, 4 named each). The **key and button binding screens**: `OnSetCurrent` focuses `mgCustomizeKeys`, the arrow handlers scroll it. The two classes have identical code (same function sizes); the difference is in the gadget's data (keyboard and mouse against controller). The gadget class is the runtime `CMenuGadgetCustomizeKeys` (`menu-gadget-members.md`: items of a command name and a command index, candidate controls with device names, the active item, the first item drawn, a "binding being added" flag, a restore item). Default bindings are not in the layouts (`gui.md`); `CMSControllerOptions`' presets are.

### 8. `CMSRenovationOptions` (Sam2Renovation tree; no layout in the base game)

The class has a layout **only in `Content/Sam2Renovation/Menus/CMSRenovationOptions.rsc`** (11 gadgets, class `CMSRenovationOptions`; script section 8: **8 of 8 handlers match its gadgets**): `opslEnableRagdollDeaths` (`YES`/`NO`, default `YES`), `opslEnableDebris` (default `YES`), `opslMaxDebris` (8 entries `15` to `50` in steps of 5, default `40`), `opslEnableUziScreens` (default `NO`), `opslEnableUziReloading` (default `YES`), `opslLODBias` (`0`, `-0.5`, `-1`, `-1.5`, `-2`; default `-1`), `btnDefaultSettings`. Handlers: six `_OnChange` of 5 bytes (empty), `SyncOptionsToGadgets` / `SyncOptionsFromGadgets` (649 and 639 bytes: the six values with the `sam_` debris and ragdoll variables, `console-config.md` lists "maximum debris (20)" as the base default), `btnDefaultSettings_OnClick/_OnYes`. The mod's options hub (`cmsoptionsselection` of the same tree) links to it as `lnkRenovationOptions`.

### 9. `CGameOptionsMenuScreen`

No method, **no layout in any tree** (script section 8). The data has a layout `coptionsmenuscreen` (class `SOptionsMenuScreen`: music and effects volume, picture gamma, blood colour `RED`/`GREEN`/`HIPPIE`/`NO BLOOD`, gore, language `ENGLISH`/`GERMAN`/`ITALIAN`/`KOREAN`) that no exported class names by stem: probably the older options screen this class belonged to (name evidence, low: T01-205).

## Invariants

1. Every `_OnChange` of this page names an option, slider or integer select box of its own layout (script: 105 of 105 over the family; on this page `CMSAudioOptions.sldVoice` is the one exception).
2. The video quality list has 6 entries and the preset table 5 levels; the display mode list has 4 entries in the order full screen, windowed, borderless, maximised, which is the order of the code's mode indices 0, 1, 2, 3 (script and read).
3. The connection presets (3,000/12,000; 5,000/65,536; 7,500/100,000) lie inside the integer boxes' ranges and the third equals the defaults of the boxes and of `cli_iMaxOutBPS` / `cli_iMaxBPS` (script and `replication.md`).
4. The enemy multiplier percentages are **not** on this page (`menu-classes-session.md`).
5. All 36 variable names of the preset table and the fixed list are listed in `console-variables.md` (checked by hand against that page: 36 of 36).

## Open questions

1. The variable behind each gadget of the video, advanced video, audio, game and controller screens (only the 36 above and the two bandwidth variables are named): T01-196.
2. The option-to-value mapping of the six advanced preset groups (for example the five `opslDynamicShadows` presets and which of the 28 variables each sets): T01-196.
3. The units of `tex_iMaxSize` and `lmp_iMaxSize` presets (10 to 14 against the default 8): T01-196.
4. Which popup raises `msgKeepSettings` and its time limit: T01-203.
5. The meaning of the fifth connection entry and of `opslMouseLook`: T01-196.
6. `CGameOptionsMenuScreen` and `coptionsmenuscreen`: T01-205.

## Test vectors

1. **Quality level 2** (read): `SyncCvarsFromQuality(2)` sets `tex_iMaxSize` = `lmp_iMaxSize` = 12, `gfx_iAntiAliasing` 1, `tex_iAnisotropy` 2, `tex_fLODBias` -3, `ren_fLODBias` 0.75, `ren_pixDynamicShadowSize` 256, `sha_bAllowRefractions` 1, `d3d_iMaxAllowedPixelProgramVersion` 30. If a game then changes only `ren_fLODBias` to 0.8, `SyncCvarsToQuality` finds no level and selects option 5 `CUSTOM`.
2. **Level clamp** (read): `SyncCvarsFromQuality(-3)` behaves as level 0 and `SyncCvarsFromQuality(9)` as level 4.
3. **Network** (read): variables out 5,000 and in 65,536 select option 1 (`ISDN`) with the integer boxes disabled; out 6,000 and in 80,000 select option 3 (`CUSTOM`), boxes enabled; the default button selects option 2 (`CABLE/ADSL`) with 7,500 and 100,000.
4. **Display mode** (read, data): full screen on gives mode 0; windowed with the maximised flag gives 3; simulated full screen gives 2; none of them gives 1 (`FULL SCREEN`, `WINDOWED`, `BORDERLESS`, `MAXIMIZED` at indices 0, 1, 2, 3 in the layout).
