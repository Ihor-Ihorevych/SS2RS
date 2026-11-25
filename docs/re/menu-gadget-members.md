# Menu gadget classes: the layout classes in the data and the runtime gadget members (T01-112, companion page)

- **Roadmap task:** T01-112 (overview: [`menu-classes.md`](menu-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the member names of the layout classes below (the binary's declared names matched to the data's numbered members by count, order and type in `tools/re/menu_classes_verify.py` section 4: 22 of 25 compared classes agree in full; the three exceptions are explained in section 2) and for the menu parameter values (section 3); **medium** for the meaning of a member read from its name only; **low** for the runtime gadget classes (names only, no code read).

Prerequisites: [`menu-classes.md`](menu-classes.md), `gui.md`, `rsc-ctl.md`, `conventions.md` (the order rule: the binary declares every member name in a table in the order of the data's members; offsets are not recorded).

## Summary

**None of the runtime gadget classes is exported** by any module; only the screens are (104 classes). What `Sam2Game.dll` does reveal are the **declared member tables** of the gadget classes: the layout classes (the saved part of a gadget: 25 classes with a type table in the `Menus/*.rsc` files) and the runtime gadget objects (the state of a gadget on screen). This page lists both. The result is that **the numbered members of the layout classes now have names**, which `gui.md` could describe only by their types.

## Evidence

The binary's member tables were read (names, types, order) for the 25 layout classes below, for the menu parameters and for 10 runtime classes; the data side is the type table of each class in the 99 layouts and `Databases/MenuLayout.rsc`. `tools/re/menu_classes_verify.py` (sections 4 and 9) compares them. **Not checked:** any function of the gadget classes (not exported), the drawing, the focus and the input handling.

## Layout / rules

### 1. The layout classes (data) with the binary's declared names

Notation: `.n` is the data's own member number (the data names only about half of the members; the **file order** of the members is the declaration order, so the numbers are not always ascending: `CMenuGadgetSelectBoxLayout` has `.2 .3 .4 .5 .8 .7`). The type is the data's type name. "Files" is the number of `Menus/*.rsc` files whose type table holds the class. The binary declares the same tree with the short names `CMGListLayout`, `CMGSelectBoxLayout`, `CMGButtonLayout` for `CMenuGadgetListLayout`, `CMenuGadgetSelectBoxLayout`, `CMenuGadgetButtonLayout`.

| Class (data) | Data base | Files | Data members (type) = declared name |
|---|---|---|---|
| `CMenuGadgetLayout` | None | 98 | `.1` IDENT = `mgl_idName`; `.2` Vector2f = `mgl_vPos`; `.3` Vector2f = `mgl_vSize`; `.4` CMenuText = `mgl_mtxtGadgetText`; `.5` COLOR = `mgl_colColor`; `.6` BOOL = `mgl_bHidden`; `.7` EPlatform = `mgl_plaPlatform` |
| `CMenuText` | None | 98 | `.1` CTransString = `mtxt_strText`; `.2` IDENT = `mtxt_idFontTemplate`; `.5` ETextAlignHorizontal = `mtxt_taTextAlignH`; `.6` ETextAlignVertical = `mtxt_taTextAlignV`; `.4` Vector2f = `mtxt_vTextOffset` |
| `CMenuControllerButton` | None | 78 | `.1` EMenuButtonTypes = `mcb_mbtButtonType`; `.2` CTransString = `mcb_strText`; `.3` Vector2f = `mcb_vOffset`; `.4` Vector2f = `mcb_vSize`; `.5` FLOAT = `mcb_fStretch` |
| `CMenuScreenLayout` | CResource | 99 | `.1` IDENT = `msl_idName`; `.3` CString = `ms_strUnderlay`; `.4` CString = `ms_strOverlay`; `.2` `CDynamicContainer<CMenuGadgetLayout>` = `?` |
| `CMenuGadgetImageLayout` | CMenuGadgetLayout | 74 | `.1` `Ptr<CTexture>` = `mgl_ptexTexture`; `.2` BOOL = `mgl_bFlipX`; `.3` BOOL = `mgl_bFlipY` |
| `CMenuGadgetLabelLayout` | CMenuGadgetLayout | 90 | `.1` BOOL = `mgl_bBackground`; `.2` COLOR = `mgl_colBackgroundColor`; `.3` BOOL = `mgl_bOutline`; `.4` COLOR = `mgl_colOutlineColor`; `.5` BOOL = `mgl_bStrikeThrough`; `.6` FLOAT = `mgl_fStrikeHeight`; `.7` COLOR = `mgl_colStrikeBegin`; `.8` COLOR = `mgl_colStrikeEnd` |
| `CMenuGadgetButtonBarLayout` | CMenuGadgetLayout | 78 | `.2` CMenuControllerButton[4] = `mgl_amcbLeftSideButtons`; `.3` CMenuControllerButton[4] = `mgl_amcbRightSideButtons`; `.4` FLOAT = `mgl_fPadding` |
| `CMenuGadgetLinkLayout` | CMenuGadgetButtonLayout | 19 | `.1` IDENT = `mgl_idLink` |
| `CMenuGadgetSelectBoxLayout` | CMenuGadgetLayout | 21 | `.2` Vector2f = `mgl_vBoxSize`; `.3` Vector2f = `mgl_vArrowSize`; `.4` Vector2f = `mgl_vArrowOffset`; `.5` IDENT = `mgl_idValueFont`; `.8` ETextAlignHorizontal = `mgl_taValueAlignment`; `.7` FLOAT = `mgl_fPadding` |
| `CMenuGadgetOptionSelectBoxLayout` | CMenuGadgetSelectBoxLayout | 20 | `.1` `CStaticStackArray<CTransString>` = `mgl_astrOptions`; `.2` INDEX = `mg_iDefaultValue` |
| `CMenuGadgetSliderBoxLayout` | CMenuGadgetSelectBoxLayout | 6 | `.1` ULONG = `mg_ulDefaultValue`; `.2` ULONG = `mg_ulStep`; `.3` INDEX = `mg_ctMarkers`; `.4` FLOAT = `mg_fValueMultiplier`; `.5` INDEX = `mg_fValueAdd` |
| `CMenuGadgetIntegerSelectBoxLayout` | CMenuGadgetSelectBoxLayout | 6 | `.1` SLONG = `mgl_slMinValue`; `.2` SLONG = `mgl_slMaxValue`; `.3` SLONG = `mg_slDefaultValue`; `.4` SLONG = `mgl_slUpDnIncrement` |
| `CMenuGadgetImageSelectBoxLayout` | CMenuGadgetSelectBoxLayout | 1 | `.1` `CStaticStackArray<CTransString>` = `mgl_astrOptions`; `.2` INDEX = `mg_iDefaultValue`; `.3` to `.22` `Ptr<CTexture>` = `mgl_ptexTexture00` to `mgl_ptexTexture19` (20 pictures); `mgl_iColor` INDEX and `mgl_fCrosshairSize` FLOAT (the data names these two itself) |
| `CMenuGadgetListLayout` | CMenuGadgetLayout | 11 | `.1` Vector2f = `mgl_fArrowSize`; `.2` Vector2f = `mgl_fArrowOffset`; `.3` `CDynamicContainer<CListColumnInfo>` = `mgl_clciColumnInfos`; `.7` FLOAT = `mgl_fRowPadding`; `.11` FLOAT = `mgl_fRowHeight`; `.12` COLOR = `mgl_colRowBackground`; `.9` IDENT = `mgl_idIndexFont`; `.10` BOOL = `mgl_bDrawMargins`; `.13` CMGVerticalSliderLayout* = `mgl_pmglVerticalSlider`; `.14` IDENT = `mgl_idCaptionFont` |
| `CListColumnInfo` | None | 11 | `.1` FLOAT = `lci_fWidth`; `.5` ETextAlignHorizontal = `mgl_taValueAlignment`; `.4` IDENT = `mgl_idValueFont`; `.6` CTransString = `lci_strCaption`; `.11` ETextAlignHorizontal = `lci_taHorizontal`; `.12` ETextAlignVertical = `lci_taVertical` |
| `CMenuGadgetChapterListLayout` | CMenuGadgetListLayout | 1 | `.1` IDENT = `mgl_idCaptionFont` |
| `CMenuGadgetFriendsListLayout` | CMenuGadgetListLayout | 1 | `.1` IDENT = `mgl_idCaptionFont`; `.2` CMGStatusBarLayout* = `mgl_pmglStatusBarLayout` |
| `CMenuGadgetOptiMatchListLayout` | CMenuGadgetListLayout | 1 | `.1` IDENT = `mgl_idCaptionFont`; `.2` BOOL = `mgl_bShowPing` |
| `CMenuGadgetServerListLayout` | CMenuGadgetListLayout | 1 | `.1` IDENT = `mgl_idCaptionFont` |
| `CMenuGadgetScrollListLayout` | CMenuGadgetLayout | 8 | `.3` INDEX = `mgl_iActiveItem`; `.4` FLOAT = `mgl_fScrollSpeed`; `mgl_ctColumns` INDEX |
| `CMenuGadgetVerticalSliderLayout` | CMenuGadgetLayout | 11 | `.1` Vector2f = `mgl_vArrowSize`; `.2` Vector2f = `mgl_vArrowOffset`; `.3` COLOR = `mgl_colBackground` |
| `CMenuGadgetProgressBarLayout` | CMenuGadgetLayout | 5 | `.1` COLOR = `mgl_colBorder` |
| `CMenuGadgetRenderModelLayout` | CMenuGadgetLayout | 2 | `.1` CString = `mgl_strModelPath`; `.3` CMGRenderModelOrientation = `mgl_moOrientation`; `.2` FLOAT = `mgl_fDistance`; `.4` BOOL = `mgl_bRotation`; `.6` FLOAT = `mgl_fRotSpeedH`; `.5` FLOAT = `mgl_fFOV`; `.7` CString = `mgl_strAnimation`; `.8` BOOL = `mgl_bBackgroundFoil`; `.9` CMGRenderModelFoil = `mgl_mfFoil` |
| `CMGRenderModelOrientation` | None | 2 | `.1` FLOAT = `mgrmo_fHeading`; `.2` FLOAT = `mgrmo_fPitch`; `.3` FLOAT = `mgrmo_fBanking` |
| `CMGRenderModelFoil` | None | 2 | `.1` Vector2f = `mgmf_vSize`; `.2` COLOR = `mgmf_colFoil` |


### 2. Where the binary and the data differ (the three classes that do not match in full)

- **`CMenuScreenLayout`**: the data has 4 members (`.1` name, `.3` underlay, `.4` overlay, `.2` gadget list); the binary declares 3 (`msl_idName`, `ms_strUnderlay`, `ms_strOverlay`). **The gadget list has no declared name**; the three named members sit at their positions.
- **`CMenuGadgetListLayout`**: the names `mgl_fArrowSize` and `mgl_fArrowOffset` say float, but the data's type is `Vector2f` (two numbers each); the binary's `CMenuGadgetVerticalSliderLayout` and `CMenuGadgetSelectBoxLayout` use `mgl_vArrowSize` and `mgl_vArrowOffset` for the same kind of member. **The data's type is the truth** (high); the list's names are loose.
- **`CMenuGadgetSliderBoxLayout.5`** (`mg_fValueAdd`) is typed `INDEX` in the data (a whole number) although the name begins with `f`; `.4` `mg_fValueMultiplier` is a float. In every slider box of the data both are -1 (script section 5 prints the five members of all 15), so the data never shows what they do; the pair looks like a linear map "value times multiplier plus add" for the displayed number (name evidence, low).
- **`CListColumnInfo`** matches in count and order but its names are borrowed (`mgl_taValueAlignment` and `mgl_idValueFont` at `.5` and `.4`: read as the alignment and font of the column's values; `.11` and `.12` the horizontal and vertical alignment of the column **caption**).
- **The base members** `CMenuGadgetLayout.1` to `.7` are: **name, position, size, text, colour, hidden flag, platform**. `.6` is a **hidden** flag, not a "visible" flag (`mgl_bHidden`; read: 0 = shown, name evidence). `.7` `mgl_plaPlatform` is the platform mask that `gui.md` decoded from the data (values 3, 1, 2): **the declared name independently confirms** that member's meaning.

### 3. The menu parameters and the layout database (data, `Databases/MenuLayout.rsc`)

The file `Content/SeriousSam2/Databases/MenuLayout.rsc` holds one **`CMenuLayoutDatabase`** (8 data members; three declared names are known without position: `mld_mpaMenuParameters`, `mld_idDefaultFontTemplate` and `mld_iGridSizeIndex`; the data shows the default font template `SeriousZurich`, the grid sizes `[1, 5, 10, 20]` with index 0, and 10 font instances) and the **`CMenuParameters`** record (7 members, names declared in the binary, data types `SLONG` x6 and `BOOL`: script section 9):

| Member | Value | Meaning (by the name) |
|---|---|---|
| `mep_slTopBarHeight` | 150 | height of the top bar of the screen frame, in the menu's virtual canvas units |
| `mep_slBottomBarHeight` | 128 | height of the bottom bar |
| `mep_slImageTop` | 97 | top of the image area |
| `mep_slImageBottom` | 402 | bottom of the image area |
| `men_slOptionsFontHeight` | 24 | font height of the options (select boxes) |
| `men_slLabelsFontHeight` | 15 | font height of labels |
| `men_bRenderSafeArea` | 0 | draw the **safe area** frame (a television margin); 0 = off |

The units are not stated by the data; the layouts place gadgets inside a canvas of about 640 by 480 (confidence medium: `gui.md` open question on the virtual canvas, T10-017), consistent with a 150-high top bar.

### 4. Runtime gadget classes (declared members only; no code read; low)

The binary declares these runtime classes (the object a layout becomes on screen). The names carry the type by their prefix; the declared bases are the short names of the non-exported base classes.

| Runtime class | Base | Declared members |
|---|---|---|
| `CMenuGadgetImage` | `CMenuGadget` | `mg_ptexTexture` (the texture) |
| `CMenuGadgetProgressBar` | `CMenuGadget` | `mg_fCurrentPercentage` (0 to 1; `CPopupMenuPleaseWait::Step` writes the bar's value) |
| `CMenuGadgetVerticalSlider` | `CMenuGadget` | `mg_fMarkerStart`, `mg_fSliderSpan`, `mg_fPageSpan` (the thumb start, the slider length, the page length) |
| `CMenuGadgetList` | `CMenuGadget` | `mg_clrdRows` (the rows), `mg_iSelectedRowIndex`, `mg_iFirstDisplayedRow`, `mg_ctNumOfDisplayedRows`, `mg_pmgVerticalSlider` (the slider it owns) |
| `CMenuGadgetFriendsList` | `CMGList` | `mg_pmgStatusBar` |
| `CMenuGadgetIntegerSelectBox` | `CMGSelectBox` | `mg_slCurrentValue` |
| `CMenuGadgetOptionSelectBox` | `CMGSelectBox` | `mg_iCurrentValue` (the index that every `_OnChange` handler switches on and that `GetBOOLValue` compares with 0) |
| `CMenuGadgetSliderBox` | `CMGSelectBox` | `mg_ulCurrentValue` |
| `CMenuGadgetCustomizeKeys` | `CMenuGadget` | `mgck_ackiCustomKeyItems` (the custom key items), `mgck_ackcControlCandidates` (the candidate controls), `mgck_iActiveItem`, `mgck_iFirstToRender`, `mgck_bAddingBinding`, `mgck_ckiRestore` (the item to restore on cancel) |
| `CMGEnterName` | `CMenuGadget` | `en_abiButtons` (the on-screen keyboard's button items; each has a position, a size, a back and a front colour, a content and a description string, a "command" flag and a "name holder" flag) |

**Value classes (declared members).** The custom key candidate (`ckc_strControlName`, `ckc_strControlDevice`, `ckc_strModControlName`, `ckc_strModControlDevice`, `ckc_strTransControlName`, `ckc_strModTransControlName`: a control, its device, the modifier and the translated texts) and the custom key item (`cki_strCommandHRF` the command in human-readable form, `cki_iCommandIndex`). The list item classes are on [`menu-classes-play.md`](menu-classes-play.md) (the save list row 11 members, the demo row 3, the player list row `lrd_iPlayerIndex`, the level items `msli_strWorldPath` and `mselei_strWorldPath`, the character item 4 members, the achievement item 4). `CMenuScreenData` has one member `msd_str` (a string; the screen variable store carries such values). The one base-screen member known by name is **`ms_idCallerGadget`** (the gadget that opened the screen, `menu-classes.md` section 5). The screen classes with declared members: `CMSControllerOptions` 6, `CMSNetworkOptions` and `CMSGamespy` 2 each (`ms_iUpstreamBPS`, `ms_iDownstreamBPS`), `CMSOnlineVoicePlayback` 6, `CPopupMenuPleaseWait` 7 (each on its page) and the exported static `CMainMenu::ms_bInitialInteraction`.

## Invariants

1. For each of the 22 fully matched classes the data's member count equals the binary's declared count, every member the data names itself (for example `mgl_ctColumns`, `mgl_iColor`) is at its declared position, and every type fits the name's prefix (script section 4).
2. The platform mask is `mgl_plaPlatform`; the hidden flag is `mgl_bHidden`.
3. The connection presets and the menu parameters are consistent with the integer boxes and canvas of the layouts (`menu-classes-options.md`).

## Open questions

1. The runtime classes' functions (not exported): focus, repeat, drawing: T01-197 (with T01-078).
2. The classes with a data type table but no declared member table found (the arranger `CMGArrangerLayout`, the image button, name entry, custom keys, background gadgets, status bars, online list layouts), and the `CMenuLayoutDatabase` members without a declared name: T01-204.
3. What the slider `-1` multiplier and add do: T01-204.

## Test vectors

1. `CMenuGadgetLayout` of `Menus/CMainMenu.rsc`: the data's `.7` (platform) is 3 for `lnkSinglePlayer`, 1 for `btnMultiplayer` and 2 for `btnXboxLive` (the declared name `mgl_plaPlatform`; `gui.md` counts 778, 55 and 12 over all gadgets).
2. `CMenuParameters`: top bar 150, bottom bar 128, image area 97 to 402, fonts 24 and 15, safe area off.
