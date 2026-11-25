# HUD parameter classes and the HUD parameter database (T01-111, part of T01-026)

- **Roadmap task:** T01-111 (part 11 of the class inventory T01-026; overview: `hud-classes.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/hud_classes_verify.py` reproduces from the data: the 44 entries of `Databases/HudParams.rsc`, their classes, bases, positions, flags, the shared configuration records, the textures, the 11 members of `SimpleHudBarParams.rsc`, and the **member names** (31 classes, 141 binary-declared members, all agree with the data on member count, on the position of every member the data names and on the data types, section 2); **medium** for the meaning of a parameter that rests on its name or on one reading of the code that uses it (marked); **low / open** for pixel-exact drawing rules (parked, `hud-classes.md` open questions).

Prerequisites: `hud-classes.md` (overview, naming, enumerations), `rsc-ctl.md` and `rsc-ctl-schema.md` (the `.rsc` container and the generated member reference), `ctsemeta-types.md`, `pfx.md` (the `CGradient` / colour-key records), `fnt.md` (fonts).

## Summary

The HUD's data is two `.rsc` files:

- **`Databases/HudParams.rsc`** (22,696 bytes, one `CHudElementsParamsDatabase`): a shared block of four numbers (`CCommonSimpleHudElmParams`, the number shake) and a **list of 44 parameter objects, one per HUD element class**, each of a different `S`/`C...HudElementParams` class. Position, render mode, textures and layout of every HUD element come from here.
- **`Databases/SimpleHudBarParams.rsc`** (2,477 bytes, one `CSimpleHudBarParams`): the ten bar textures and the number font that all 22 "simple" elements share.

Both files are the **only** places in the 8,063 format-8 files where any of the 47 saved HUD classes occurs (every saved class is in exactly 1 file; `--all`). The 45 parameter classes of the family are saved: **42 are entries of the database** (plus the two of `players-session-classes.md`: `CPlayerDeathHudElementParams`, `CVotingInfoHudElementParams`, 44 in all), **3 are abstract bases** that no entry uses (`SHudElementParams`, `SSimpleHudElementParams`, `SAmmoHudElementParams`). Three small records (`SHudNumberConfig`, `SHudIconConfig`, `SHudBarConfig`) are the text, icon and bar configurations that the simple elements point at.

## Evidence

- **Data.** `tools/re/hud_classes_verify.py` (standard library, read-only; the class table of section 1 over all 8,063 format-8 files with `--all`, about 4 minutes, over the 112 `.rsc` files without) decodes both files with the value-tree decoder of `ctse_values.py` and prints every number quoted below. The `Content/SeriousSam2` paths are the install's.
- **Binary.** The member names come from the static member tables of `Sam2Game.dll` (name, type reference, flags; offsets read but **not recorded**, `conventions.md`). The tables were found from the class registrations (parent name, editor caption, data name). The data's own type tables name some members and number the others; the binary names every one. The **member-order rule** of `weapon-classes.md` section 4.0 is checked here on 31 data classes (section 2): same count, every data-named member at the position of the same binary name, every data type consistent with the Hungarian prefix of the binary name (one exception, section 2). A number-only member of the data is therefore given the name at its position.
- **Use of a parameter** (`rule`, section 6) rests on reading the small methods that read the parameters (**medium**, one reading each); where the data agrees (for example positions inside the 640 x 480 canvas) the table says so.
- **Not checked:** nothing was run; no image of the HUD was produced or compared.

## Layout / rules

### 1. The database object

```
CHudElementsParamsDatabase                   (base CResource; 2 members)
  .2  hed_chpCommonSimpleHudElmParams        CCommonSimpleHudElmParams (4 floats)
  .1  hed_cepHudElementsParams               list of 44 CScriptObject (each an object of a *HudElementParams class)
CCommonSimpleHudElmParams                    (no base; 4 members; in the file once)
  .1  cshep_fShakeFreqX      30      .2  cshep_fShakeFreqY   42
  .3  cshep_fShakeDamp        5      .4  cshep_fBarStretchY   0.5
```

The list order (**high**): Lives, Armor, Health, Air, Adrenaline, Frozen, Boss, Time, Counter, ComboWeapons, Strength, Jump, SeriousDamage, SeriousScore, SeriousSpeed, Invulnerability, Invisibility, Bombs, Vehicle, VehicleAmmo, Points, Scanner, Damage, WeaponsList, HorizontalWeaponsList, WeaponsMenu, BackTemplate, LeftAmmo, RightAmmo, Conversation, NetricsaIcon, Netricsa, Freezing, OnlineNotification, ObjectAction, Saving, AutoAim, ExtraLife, ItemPicked, VotingInfo, FragsCounter, TimeRemain, PlayerDeath, EnemyPurged. **All 44 classes are different**; a lookup returns the first entry of the wanted class and the order has no other meaning that was found (the order in which the elements are created is a different list, `hud-classes.md` section 5).

The four shake numbers are used by the text of every simple element: when the text changes, the number shakes for a short time (section 6, **medium**: the names say frequency x, frequency y, damping, and the bar stretch).

### 2. Member names: what the member-order rule gives (`tools/re/hud_classes_verify.py`, section 2)

All numbers are over the **31 data classes** whose member table the binary also declares (141 binary members; the 96 members of `CNetricsaHudElementParams` are named in the data itself and are not counted):

- the member **count** is the same in the data and the binary for **31 of 31**;
- every member the data names is at the position of the same name in the binary for **31 of 31**;
- the data type fits the Hungarian prefix of the binary name for **140 of 141** members; the exception is `hwlhep_fTextOffset` of `SHorizontalWeaponsListHudElementParams`, whose data type is a `Vector2f` (the `f` prefix of the name is wrong, the data type wins);
- **114 data members** are number-only and get their name from this rule.

**Confidence: high** for the names (script), **medium** for the one-line meaning given beside a name when it is only the name.

### 3. The base records

| Class (data name) | Members, in the data's order (data id → binary name · type) | Notes |
|---|---|---|
| `SHudElementParams` (base `CScriptObject`; exported `CHudElementParams`) | `.1` → `hep_vPosition` · `Vector2f`; `hep_rheRender` · `RenderHudElement`; `hep_bRenderOnCamera` · `BOOL` | **position** in the 640 x 480 canvas, **render mode** (enumeration, 44 of 44 entries have 1 = `RHE_AUTO`), **render on camera** (6 of 44 entries have 1: Invisibility, Conversation, NetricsaIcon, OnlineNotification, Saving, VotingInfo) |
| `SSimpleHudElementParams` (base `SHudElementParams`; `CSimpleHudElementParams`) | `.5` → `shep_bRelToPrevious` · `BOOL`; `.4` → `shep_hhtcTextCfg` · `Handle<CHudTextConfig>`; `.2` → `shep_hhicIconCfg` · `Handle<CHudIconConfig>`; `.3` → `shep_hhbcBarCfg` · `Handle<CHudBarConfig>`; `.6` → `shep_pshupBarParams` · `Ptr<CSimpleHudBarParams>` | the common part of the 22 simple elements; **14 of 22** have `bRelToPrevious` set (the stacking flag, `hud-classes.md` section 8); a handle is **null** when the element has no such part (text 4 null of 22, icon 1 null, bar 5 null); the bar-parameters pointer is set in **21 of 22** and always the same object (`SimpleHudBarParams.rsc`) |
| `SAmmoHudElementParams` (base `SSimpleHudElementParams`; `CAmmoHudElementParams`) | `.1` → `ahep_vIconRelPos` · `Vector2f`; `.3` → `ahep_fIconStretch` · `FLOAT`; `.2` → `ahep_bShowIcon` · `BOOL`; `.4` → `ahep_vTextRelPos` · `Vector2f`; `.5` → `ahep_fTextSize` · `FLOAT`; `.6` → `ahep_bShowText` · `BOOL`; `ahep_haHorizontalAlign` · `HorizontalAlign` | used by the two ammunition entries only; the data order of the first six is `.1 .3 .2 .4 .5 .6` and the binary order is icon position, icon stretch, show icon, text position, text size, show text |

The **configuration records** (`CHudTextConfig` = data `SHudNumberConfig`, `CHudIconConfig` = `SHudIconConfig`, `CHudBarConfig` = `SHudBarConfig`; `CObject`-less value records saved inside the file):

| Record | Members (data id → binary name · type) | Count in the file |
|---|---|---|
| `SHudNumberConfig` (text) | `.1` → `htc_vRelPos` · `Vector2f`; `.3` → `htc_haHorizontalAlign` · `HorizontalAlign` (0 left, 1 centre, 2 right); `.4` → `htc_fShakePower` · `FLOAT` (4 or 0) | 20 |
| `SHudIconConfig` (icon) | `.1` → `hic_vRelPos`; `.4` → `hic_fStretch` (0.185 to 1); `.3` → `hic_ptxTexture` · `Ptr<CStaticTexture>` (null in 3 of 21) | 21 |
| `SHudBarConfig` (bar) | `.1` → `hbc_vRelPos`; `.2` → `hbc_fTilingHolderLength` (10, 15, 20, 30); `.7` → `hbc_fBarLength` (70, 120, 200); `.8` → `hbc_colHolder` (ARGB, `pfx.md`); `.9` → `hbc_bFlipH`; `.10` → `hbc_grPhase1` and `.11` → `hbc_grPhase2` (two `CGradient`, the bar colour over its fill, one key to three keys) | 17 |

The names of the two gradients (`Phase1`, `Phase2`) point to the two phases of the bar colour hook (`AdjustColorForPhase1/2`, `hud-classes-simple.md` section 2) (**medium**, name).

### 4. The 24 entries below `SSimpleHudElementParams` (all numbers from the file; the canvas is x 0..640, y 0..480)

"Cam" = render on camera, "Stack" = `bRelToPrevious`. Offsets are relative to the entry's position. Colours are the holder colour as 32-bit ARGB (alpha in the top byte, `pfx.md`). Text align L/C/R is 0/1/2.

| # | Entry class (data name) | Position | Cam | Stack | Text: offset / align / shake | Icon: offset / stretch / texture | Bar: offset / holder / length / holder colour |
|---|---|---|---|---|---|---|---|
| 1 | `SLivesSimpleHudElementParams` | (12, 27) | 0 | 0 | (18.22, 33.9) / L / 4 | (-15, 0) / 0.25 / sam_life | - |
| 2 | `SArmorSimpleHudElementParams` | (5, 397) | 0 | 0 | (30, 7) / L / 4 | (0, 0) / 0.2 / armor | (24, 14) / 10 / 120 / 0xFFDFDFDF |
| 3 | `SHealthSimpleHudElementParams` | (5, 429) | 0 | 0 | (30, 7) / L / 4 | (0, 0) / 0.2 / health | (24, 14) / 10 / 120 / 0xFFFF0000 |
| 4 | `SAirSimpleHudElementParams` | (195, 55) | 0 | 1 | - | (0, 0) / 0.25 / oxygen | (30, 20) / 30 / 200 / 0xFF82A0C5 |
| 5 | `SAdrenalineSimpleHudElementParams` | (195, 55) | 0 | 1 | - | (0, 0) / 1 / adrenaline | (30, 20) / 30 / 200 / 0xFFF9F88E |
| 6 | `SFrozenSimpleHudElementParams` | (195, 45) | 0 | 1 | - | (-2, 8) / 0.2 / frost | (30, 20) / 15 / 200 / 0xFFDFDFDF |
| 7 | `SBossSimpleHudElementParams` | (210, 45) | 0 | 1 | (40, 10) / L / 4 | (4.723, 6.92) / 0.19 / (override) | (30, 20) / 20 / 200 / 0xFFDFDFDF |
| 8 | `STimeSimpleHudElementParams` | (195, 55) | 0 | 1 | (40, 10) / L / 4 | (0, 0) / 0.25 / clock | (30, 20) / 30 / 200 / 0xFFDFDFDF |
| 9 | `SCounterSimpleHudElementParams` | (195, 45) | 0 | 1 | (40, 10) / R / 0 | (0, 0) / 1 / Counter | (30, 20) / 30 / 200 / 0xFFDFDFDF |
| 10 | `SComboWeaponsSimpleHudElementParams` | (195, 45) | 0 | 1 | (40, 4) / L / 4 | (-3, 5) / 0.225 / ComboWeapons | (30, 20) / 10 / 70 / 0xFFDFDFDF |
| 11 | `CStrengthSimpleHudElementParams` | (4, 100) | 0 | 1 | (35, 12) / L / 4 | (-3, 5) / 0.225 / SeriousStrength | (30, 20) / 10 / 70 / 0xFFDFDFDF |
| 12 | `CJumpSimpleHudElementParams` | (4, 100) | 0 | 1 | (35, 12) / L / 4 | (-3, 5) / 0.225 / SeriousJump | (30, 20) / 10 / 70 / 0xFFDFDFDF |
| 13 | `CSeriousDamageHudElementParams` | (4, 100) | 0 | 1 | (35, 12) / L / 4 | (-3, 5) / 0.225 / SeriousDamage | (30, 20) / 10 / 70 / 0xFFDFDFDF |
| 14 | `CSeriousScoreHudElementParams` | (4, 100) | 0 | 1 | (35, 12) / L / 4 | (-3, 5) / 0.225 / SeriousScore | (30, 20) / 10 / 70 / 0xFFC2C2C2 |
| 15 | `CSeriousSpeedHudElementParams` | (4, 100) | 0 | 1 | (35, 12) / L / 4 | (-3, 5) / 0.225 / SeriousSpeed | (30, 20) / 10 / 70 / 0xFFC2C2C2 |
| 16 | `CInvulnerabilityHudElementParams` | (4, 100) | 0 | 1 | (35, 12) / L / 4 | (-3, 5) / 0.225 / Invulnerability | (30, 20) / 10 / 70 / 0xFFDFDFDF |
| 17 | `CInvisibilityHudElementParams` | (4, 100) | 1 | 1 | (35, 12) / L / 4 | (-3, 5) / 0.225 / Invisibility | (30, 20) / 10 / 70 / 0xFFDFDFDF |
| 18 | `SBombsSimpleHudElementParams` | (450, 420) | 0 | 0 | (34.85, 23.87) / R / 4 | (0, 0) / 0.225 / grenade | - |
| 19 | `SVehicleSimpleHudElementParams` | (420, 430) | 0 | 0 | (155, 2.307) / R / 4 | (150.5, 0) / 0.185 / vehicles_ATV | (39.98, 9.87) / 10 / 120 / 0xFFDFDFDF, flipped |
| 20 | `CVehicleAmmoSimpleHudElementParams` | (565, 380) | 0 | 0 | (23, 0) / R / 4 | (0, 0) / 0.2 / (override) | - |
| 21 | `SPointsSimpleHudElementParams` | (320, 32) | 0 | 0 | (0, 0) / C / 4 | (-75, -10) / 0.5 / (none) | - |
| 22 | `SLeftAmmoHudElementParams` (base `SAmmoHudElementParams`) | (510, 445) | 0 | 0 | (0, 0) / L / 4 | - | - |
| 23 | `SRightAmmoHudElementParams` (base `SAmmoHudElementParams`) | (610, 445) | 0 | 0 | (0, 0) / L / 4 | - | - |
| 24 | `COnlineNotificationHudElementParams` | (48, 98) | 1 | 0 | - | - | - |

Rows 1 to 21 and row 24 are the **22 direct children** of `SSimpleHudElementParams`; rows 22 and 23 are the two entries of base `SAmmoHudElementParams` (a child of `SSimpleHudElementParams`), whose own members carry the real layout (section 5). Row 24, the online notification, has no configuration records. With the 20 entries of section 5 the database has 22 + 2 + 20 = **44** entries (script, **high**).

Reading the table (**high** unless marked):

1. **Stacks.** The 14 entries with `Stack` = 1 form **four stacks at four positions**: (4, 100): the seven power-ups (Strength, Jump, Damage, Score, Speed, Invulnerability, Invisibility); (195, 45): Frozen, Counter, ComboWeapons; (195, 55): Air, Adrenaline, Time; (210, 45): Boss alone. The elements of one stack are drawn in the order of the entries' creation (`hud-classes.md` section 5), one under the other when they are visible (`hud-classes.md` section 8, **medium**). The other eight simple entries stand alone.
2. **Left column.** Health (5, 429) and Armor (5, 397) have the same offsets (icon 0.2, text (30, 7), bar (24, 14), holder 10, length 120) and are 32 canvas units apart; Health is the lower one.
3. **Right side.** Bombs (450, 420), Vehicle (420, 430), VehicleAmmo (565, 380) and the two ammunition entries (510, 445) and (610, 445) are all in the right bottom quarter of the canvas.
4. **Top.** Points (320, 32) is the only centre-aligned text (align 1) and has an icon record with a negative offset (-75, -10), stretch 0.5 and **no texture**; Lives (12, 27) is the top-left element.
5. **Null textures** (3 of 21 icon records): Boss (the icon comes from the boss, `GetIconTextureOverride`), VehicleAmmo (the icon comes from the vehicle) and Points (none). Vehicle also has an override at run time (the vehicle's own icon replaces `vehicles_ATV`, **medium**).
6. **Colours of the bar fill** (the two gradients) are not tabulated: the 17 bar records have one key (a flat colour) or, for Air, three keys (positions 0.23, 0.53 and 1.0).

### 5. The 20 other entries (not below `SSimpleHudElementParams`)

| Entry class (data name) | Position | Cam | Members (binary name · value in the data) |
|---|---|---|---|
| `SScannerHudElementParams` | (560, 45) | 0 | `shep_fStretch` 0.275, `shep_fRange` 150, `shep_fStretchPoints` 0.2, `shep_fStretchArrows` 0.25; colours (ARGB) `shep_colEnemy` 0xFFFF0000, `shep_colNPC` 0xF74E91F1, `shep_colNeutral` 0xFF00FF34, `shep_colObjectives` 0xF7FF9B30, `shep_colObjFailed` 0xFFFF66FF, `shep_colBeacon` 0xFFFFD800; textures `shep_ptxScanner`, `...ScannerPoint`, `...Outline`, `...ScannerDir`, `...ScannerArrow` (`Scanner.tex`, `ScannerPoint.tex`, `ScannerOutline.tex`, `ScannerDirection.tex`, `ScannerArrow.tex`); five positions `shep_vRightTop` (560, 45), `shep_vRightMiddle` (560, 205), `shep_vRightBottom` (560, 350), `shep_vCenterBottom` (285, 405), `shep_vLeftMiddle` (0, 205): the five places where the scanner can sit |
| `SDamageHudElementParams` | (0, 0) | 0 | `dhep_colDamageMin` 0xFF79183C, `dhep_colDamageMax` 0x00FF0000, `dhep_colDamageDir` 0xFF990000 (data ids `.5 .1 .2`); `dhep_fDuration` 1 (s), `dhep_fWidth` 200; `dhep_ptxDamage` = `Damage.tex`, `dhep_ptxDamageDir` = `DamageDirection.tex` |
| `SWeaponsListHudElementParams` | (580, 220) | 0 | `wlhep_fStretchY` 0.8, `wlhep_fWidth` 80, `wlhep_fWeaponOffsetX` -7.16, `wlhep_fQuantityOffsetX` 27, `wlhep_fStayTime` 2 (seconds the list stays after a change); textures `WeaponBarCorner`, `WeaponBarTile`, `WeaponBarQuantityBack`, `WeaponScreenNoAmmo`, `WeaponScreenFill`, `WeaponBarCornerGlow`, `WeaponBarTileGlow` |
| `SHorizontalWeaponsListHudElementParams` | (40, 5) | 0 | `hwlhep_fStretchY` 0.7, `hwlhep_fWidth` 60, `hwlhep_fBarOffsetX` 59, `hwlhep_fWeaponOffsetX` 27, `hwlhep_fQuantityOffsetX` 52, `hwlhep_fStayTime` 2, `hwlhep_fNumberBarStretch` 0.458, `hwlhep_fTextOffset` (27.0, -0.56), `hwlhep_colBackground` 0xFF000140, `hwlhep_fBackgroundHeight` 166.6; the seven textures of the vertical list plus `WeaponNumberBarCorner`, `...Tile`, `...CornerGlow`, `...TileGlow`; font `hwlhe_pfntFont` = `SeriousZurich.fnt` |
| `SWeaponsMenuHudElementParams` | (320, 240) | 0 | `wmhep_fStretch` 0.35, `wmhep_fOffset0FromC` 74.6 (inner ring), `wmhep_fOffset1FromC` 134.6 (outer ring), `wmhep_fArrowLighting` 0.2 |
| `SBackTemplateHudElementParams` | (5.834, 0) | 0 | `bthep_ptxBackTemplate` **null** |
| `CConversationHudElementParams` | (34, 305) | 1 | `chep_ptfxcPlayerTextFXCfg` = `PlayerConversation.tfx`, `chep_ptfxcPuppetTextFXCfg` = `PuppetConversation.tfx`, `chep_ptfxcNetricsaTextFXCfg` = `NetricsaConversation.tfx`, `chep_bRenderTextRegions` 0 |
| `CNetricsaIconHudElementParams` | (25, 270) | 1 | `nihep_ptxNetricsa` = `netricsa.tex`, `nihep_fStretch` 0.25 |
| `CNetricsaHudElementParams` | (48, 59.2) | 0 | 96 members, all named in the data (section 7) |
| `CFreezingHudElementParams` | (0, 0) | 0 | `fhep_ptxTexture` = `Frost_003.tex`, `fhep_ctMaxCount` 300, `fhep_fMinSize` 80, `fhep_fMaxSize` 150, `fhep_gradColorOverLife` (two keys: position 0.19 colour 0x00FFFFFF, position 0.99 colour 0xFFFFFFFF) |
| `CObjectActionHudElementParams` | (50, 110) | 0 | `oahep_fiActionMessage` = a `CFontInstance` (font `SeriousVerdana_HD.fnt`, scale 0.15, section 8) |
| `CSavingHudElementParams` | (50, 100) | 1 | `shep_fiMessage` = `CFontInstance` (`SeriousVerdana_HD.fnt`, scale 0.13) |
| `CAutoAimHudElementParams` | (0, 0) | 0 | `aahep_ptxTexture` = `AutoAimTarget.tex` |
| `CExtraLifeHudElementParams` | (0, 0) | 0 | `elhep_ptfxcExtraLife` = `ExtraLife.tfx` |
| `CItemPickedHudElementParams` | (0, 0) | 0 | `iphep_ptfxcPickedItem` = `ItemPicked.tfx` |
| `CVotingInfoHudElementParams` | (10, 200) | 1 | `shep_fiMessage` = `CFontInstance` (`SeriousVerdana.fnt`, scale 0.4) |
| `CFragsCounterHudElementParams` | (0, 0) | 0 | `fchep_ptfxcMessage` = `FragsCounter.tfx` |
| `CTimeRemainHudElementParams` | (0, 0) | 0 | `trhep_ptfxcMessage` = `TimeRemain.tfx` |
| `CPlayerDeathHudElementParams` | (20, 130) | 0 | `pdhep_fiMessage` (`CFontInstance`, `SeriousVerdana_HD.fnt`, scale 0.14), `pdhep_fStayTime` 3 (s), `pdhep_fFadeOutTime` 1 (s), `pdhep_ctMaxDeaths` 6 |
| `CEnemyPurgedHudElementParams` | (0, 0) | 0 | `hep_ptfxcEnemyPurged` = `EnemyPurged.tfx` |

Notes (**high** for the values, **medium** for the meanings given):

- The scanner colours are resolved by position: the data's table lists the colour members as `.4 .5 .6 .7 .9 .8`, which the binary's names read as `colEnemy colNPC colNeutral colObjectives colObjFailed colBeacon` (the five positions `shep_v...` and the five textures follow as `.15` to `.19` and `.10` to `.14`). The damage colours are likewise in the data order `.5 .1 .2` for min, max, direction.
- The duplicate id **`.9`** in `SScannerHudElementParams` (the data's table has two members with id 9: a `FLOAT` and a `COLOR`) is explained by the member-order rule: the first is `shep_fStretchArrows` (position 4) and the second `shep_colObjFailed` (position 9); a reader must key on the **position**, not on the id.
- `SWeaponsMenuHudElementParams`: the data ids are `.1 .6 .7 .8` (not `.1 .2 .3 .4`); positions give the four names above.
- The text effects (`.tfx`) are the engine's text-effect configurations (`fx.md`, `cutscenes.md`); the HUD only passes them to a text queue.
- The two `shep_fiMessage` members (Saving and VotingInfo) and `oahep_fiActionMessage` and `pdhep_fiMessage` are `CFontInstance` records (16 members; first is the font reference, the second the colour, `fnt.md`); the four records differ only in the font file and the scale.

### 6. How the parameters are used (what the element code reads; **medium**, one reading each)

| Parameter | Used as |
|---|---|
| `hep_rheRender` | decides whether the element is visible together with the element's own wish (`hud-classes.md` section 7) |
| `hep_bRenderOnCamera` | copied into the element's `he_bRenderOnCamera`; the HUD draws the element in the "camera" pass only if it is set (`hud-classes.md` section 5) |
| `hep_vPosition` | the anchor of the element on the 640 x 480 canvas; for a stacked simple element the vertical offset of the elements drawn before it (same position, flag set) is added |
| `shep_bRelToPrevious` | the stacking flag (above) |
| `htc_vRelPos`, `htc_haHorizontalAlign` | the text is placed at the anchor plus the offset; align 1 subtracts half and align 2 the whole width of the text |
| `htc_fShakePower` | factor of the shake amplitude; the shake is a sine in x and in y with the two frequencies of the common block, multiplied by an amplitude that decays with the time since the text changed according to `cshep_fShakeDamp`, and it is switched off after a time that depends on the damping (**medium**: shape read, constants not isolated) |
| `hic_vRelPos`, `hic_fStretch`, `hic_ptxTexture` | the icon is the texture (or the element's override) scaled by the stretch and the HUD scale factor, placed at anchor plus offset |
| `hbc_*` | the bar: holder tiled to `hbc_fTilingHolderLength`, fill of length `hbc_fBarLength` multiplied by the fill fraction, mirrored when `hbc_bFlipH` is set, the two gradients give the fill colour |
| `shep_pshupBarParams` | the ten bar textures and the number font (section 9) |
| `lshep_*`, `pshep_*` | a pulse of the lives icon or the points text: for the lives icon, `lshep_fStretchingDelay` seconds after the player's `en_tmLifeAdded` the scale becomes 1 + `lshep_fExtraStretch` times a half sine over `lshep_fStretchingPeriod` seconds (and the displayed number is one lower than the real count until the delay has passed); for the points text, from the same time, 1 + `pshep_fAdditionalStretch` times a half sine over `pshep_fStretchingPeriod` seconds, only when `en_bLifeAddedByScore` is set (**medium**, one reading each; the two players' members were matched by offset) |

### 7. `CNetricsaHudElementParams` (96 members, all named in the data; the largest HUD parameter class)

The 96 members group as follows (counts by data type; **high**): 24 `FLOAT`, 14 colours, flags or integers, 18 `Vector2f` or `CMargins` records and 40 references (5 fonts and 35 textures). Examples (position 48, 59.2 on the canvas, size `nhep_vSize` 548 x 364): fonts `SeriousVerdana_MENU.fnt` for `nhep_pfntFont`, `nhep_pfntLevelName`, `nhep_pfntLevelDesc`, `nhep_pfntObjectives`, `nhep_pfntStat`, all at size 14; ring `nhep_fRingRotSpeed` 180 (degrees per second, **medium**: name); scroll speed `nhep_pixScrollSpeed` 200; margins `nhep_mLevelDescMargins` 16 on all sides and `nhep_mObjMargins` (top 53, bottom 16, left 20, right 15); shadow direction -45 and distance 0.1; 35 textures from the `Netricsa` and `Menu` folders: panel corner and tile, text box corner and tile (and the blend variants), the logo, the ring and its two rotating overlays, the letters `N`, `Etricsa` and `Net`, the bars, the sniper picture, noise, shadow, the three button bars and the PC and Xbox button pictures (`PC_Tab`, `PC_Enter`, `A`, `B`, `X`, `Y`, white, d-pad four). The full member list with every value is `rsc-ctl-schema.md` (generated). The layout rules are parked (T01-188).

### 8. The other `.rsc` objects referenced

- `CFontInstance` (16 members; five records in `HudParams.rsc` and `SimpleHudBarParams.rsc`): the font reference (`.1`), the colour (`.2`, `0xFFFFFFFF` in all five), the flags (`.3`, 0), the scale (`.6`, 0.13 to 0.4), and the numbers `.4` 0, `.5` 0, `.7` 0.75, `.8` an integer (25, 81 or 106), `.9` 1.0, `.10` 0xFF000000, `.11` 315, `.12` 0.15, `.13` 0.5, `.14` 0.5, `.15` 0, `.16` 0 (their roles belong to the font system, `fnt.md`).
- `CMargins` (4 floats: top, bottom, left, right), `CGradient` (a list of `CColorKeyFrame`: position, ARGB colour), `Vector2f`: `ctsemeta-types.md`, `pfx.md`.

### 9. `SimpleHudBarParams.rsc` (`CSimpleHudBarParams`, base `CResource`, 11 members)

| Data id | Binary name | Value |
|---|---|---|
| `.2` .. `.11` | `shbp_ptxBarFillTile`, `shbp_ptxBarFillBoxCorner`, `shbp_ptxBarFillCorner`, `shbp_ptxBarTile`, `shbp_ptxBarCorner`, `shbp_ptxBarFillCorner2`, `shbp_ptxBarFillTile2`, `shbp_ptxBarHolder`, `shbp_ptxBarHolderTile`, `shbp_ptxBarHighlight` | `BarFillTile.tex`, `BarFillBoxCorner.tex`, `BarFillCorner.tex`, `BarTile.tex`, `BarCorner.tex`, `BarFillCorner2.tex`, `BarFillTile2.tex`, `BarHolder.tex`, `BarHolderTile.tex`, `BarHighlight.tex` (all `Textures/Interface/Hud`, `Ptr<CStaticTexture>`) |
| `.12` | `shbp_fiFont` · `CFontInstance` | font `SeriousZurich_HD.fnt`, colour `0xFFFFFFFF`, scale 0.2, and 12 further numbers (the same record layout as the message fonts) |

The ten texture members are copied by every simple element into its own `hshe_ptxBar...` members when it is initialised, in the same order (**medium**, one reading). The element class also has a ten-member texture cache; the two lists have the same names in the same order (script: 10 of 10).

## Invariants

1. The database has **44** entries, all of **different classes** (script), and the render mode of every entry is **1** (`RHE_AUTO`, script).
2. All positions are **inside the 640 x 480 canvas** (script: 44 of 44), 27 distinct positions, 8 entries at (0, 0) (the ones that place their own content: scanner is not among them; Damage, Freezing, AutoAim, ExtraLife, ItemPicked, FragsCounter, TimeRemain, EnemyPurged).
3. A reader must **key a number-only member by its position** in the class's own member list (the order rule), never by its id (ids repeat, section 5).
4. A **null** configuration handle or texture means "not drawn" for that part (text, icon, bar); the element must still be created (the 22 simple entries have 4 + 1 + 5 nulls, section 3).
5. The saved names differ from the exported ones for **47 of 47** classes in two ways: 43 classes have the `S` twin (`SHealthSimpleHudElementParams` is the exported `CHealthSimpleHudElementParams`) or the same name; two classes are **renamed** (`CStrengthSimpleHudElementParams` = exported `CSeriousStrengthHudElementParams`, `CJumpSimpleHudElementParams` = `CSeriousJumpHudElementParams`; the editor captions `Serious strength` and `Serious jump` confirm the pairs).

## Open questions

1. **The shake and the pulses numerically** (the exact curves of `htc_fShakePower`, `cshep_*`, `lshep_*`, `pshep_*`): T01-187.
2. **The bar fill fractions** of the simple elements and the colour use of `hbc_grPhase1/2`: T01-187.
3. **The Netricsa layout** (the 96 members): T01-188.
4. The **weapons list layouts** (`wlhep_*`, `hwlhep_*`, `wmhep_*`): T01-190.
5. Whether the 20 "other" entries' unused members (for example `chep_bRenderTextRegions`, always 0) are ever read: T01-191.

## Test vectors

`Databases/HudParams.rsc` (22,696 bytes): `RFIL` has 85 entries (the first is `Databases/SimpleHudBarParams.rsc`), `IDNT` has one empty entry, the root is a `CHudElementsParamsDatabase` whose list `.1` holds 44 object references; the first entry (object 2) is a `SLivesSimpleHudElementParams` with position (12, 27), render mode 1, camera 0, stacking 0, a text record at (18.22, 33.9) aligned left with shake 4, an icon record at (-15, 0) with stretch 0.25 and texture `sam_life.tex`, no bar, the bar parameters `SimpleHudBarParams.rsc`, and the three lives members 1.5, 0.7, 0.6 (stretching delay, period, extra stretch).

`Databases/SimpleHudBarParams.rsc` (2,477 bytes): one `CSimpleHudBarParams`, ten texture references (`BarFillTile.tex` first), then a `CFontInstance` with font `SeriousZurich_HD.fnt` and scale 0.2.
