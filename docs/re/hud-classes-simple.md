# HUD element classes: the base classes and the 25 simple elements (T01-111, part of T01-026)

- **Roadmap task:** T01-111 (part 11 of the class inventory T01-026; overview: `hud-classes.md`; parameters: `hud-classes-params.md`; the other elements: `hud-classes-special.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, the hierarchy, the virtual-slot layout, the member lists (names from the binary's member tables) and, for each class, which data class configures it (script and registrations); **medium** for the one-sentence behaviour of every method and for what each element shows (each rests on one reading of a small function, or on the member names); **low** where a row says so. The method marks follow the earlier pages: **R** = the function was read, **P** = profiled (its callers and callees were read, not its body), **N** = name only (the sentence is the name's plain meaning). **Nothing was run.**

Prerequisites: `hud-classes.md` (naming, hierarchy, the 11 virtual slots, the container, visibility, stacking, commands), `hud-classes-params.md` (the parameters each class reads), `players-session-classes.md` (the player puppet's members the elements read; `inventory.md`).

## Summary

`CHudElement` is the 11-slot base of every HUD element; `CSimpleHudElement` adds the **bar, icon and text** machinery that **25 elements** share: 14 "simple" elements, the ammunition element and its two hand variants, the online notification and the seven power-up elements under one abstract power-up class. An element's job is: each frame, **fill in a few members** (`hshe_fBarValue1/2`, `hshe_strText`, its visibility wish) in `PrepareRenderingParameters`, and let the shared base draw them with the configuration of its parameter entry (`hud-classes-params.md`). So a simple element is described by **what it reads from the game** and **which parameter entry** it uses; the drawing is the base's.

## Evidence

- **Export table, registrations, member tables and virtual tables** as in `hud-classes.md`. For these 28 classes (the three bases `CHudElement`, `CSimpleHudElement`, `CPowerUpHudElement` and the 25 elements of section 3) every exported function and every virtual slot was listed; all 18 `PrepareRenderingParameters` of the page, `GetIconStretch`, `GetTextStretch`, `GetIconTextureOverride`, `GetPlayerHand`, `AdjustColorForPhase1`, `Render`, `RenderText`, `RenderIcon`, `Initialize`, `UpdateTemps` and `Reset` of the base were read (**R**); `RenderBar` and `RenderBarPart` were only profiled.
- **Reading the player.** The elements read members of the player puppet (`en_slHealth`, `en_slArmor`, `en_henRide`, `en_rsRidingState`, `en_fFreezingRatio`, `en_tmLifeAdded`, `en_bLifeAddedByScore`, `en_fComboWeaponsTotalDuration`, ...) and of the world info entity (`en_henBoss`, `en_strBossName`, `en_fTimerDuration`, `en_fTimerPassedTime`, `en_slCounterValue`, `en_slCounterMax`). The reading code addresses them by offset; the **names were found by matching the offset to the member tables of the player puppet and the world info entity** (the tables of `players-session-classes.md` and `mechanism-classes-world-info.md`): **medium** (one match each; the offsets are not recorded).
- **Data.** Counts and numbers: `tools/re/hud_classes_verify.py` (no data member belongs to an element class, so the script covers only the entries).
- **Not checked:** nothing was run; the bar and text formulas are **not** fully recovered: the decompiler lost the float-to-integer conversions, so the clamps of health, armour, time and counter are described only as far as they were readable (rows marked low).

## Layout / rules

### 1. `CHudElement` (the base; data name `SHudElement`; 4 members; 14 symbols = 7 standard + 7 named)

| Member | Type | Meaning (confidence) |
|---|---|---|
| `he_hhud` | handle to `CHUD` | the HUD that owns the element; the player is read through it (**high** for the type, **medium** for the use) |
| `he_bIsVisible` | `BOOL` | the result of `ShouldBeVisible` for this frame (written by `PrepareRenderingParameters`, read by the HUD's draw loop) (**medium**) |
| `he_hhepParams` | handle to `CHudElementParams` (flag 0x1000, not saved) | the element's parameter object, set by `UpdateTemps` (**medium**) |
| `he_bRenderOnCamera` | `BOOL` | copy of `hep_bRenderOnCamera` made by `Initialize` (**medium**) |

| Method | Basis | One-sentence purpose |
|---|---|---|
| `Initialize(Handle<CHUD>)` | R | store the HUD, fetch the parameters (`UpdateTemps`) and copy the render-on-camera flag |
| `UpdateTemps()` | R | look the element's parameters up in the database and keep them only if they are of the wanted class |
| `ShouldBeVisible(int wants)` | R | the visibility rule of `hud-classes.md` section 7 |
| `OnHudCommand(HudCommand)` | R | empty default |
| `PrepareRenderingParameters()` | R | empty default |
| `Reset()` | R | empty default |
| `Render(Vector2f *, float *)` | R | default draws nothing and returns 0 |

### 2. `CSimpleHudElement` (data name `SSimpleHudElement`; 21 members; 21 symbols = 7 standard + 14 named)

| Member | Type | Meaning (confidence) |
|---|---|---|
| `hshe_ptxBarFillTile`, `...FillBoxCorner`, `...FillCorner`, `...BarTile`, `...BarCorner`, `...FillCorner2`, `...FillTile2`, `...BarHolder`, `...BarHolderTile`, `...BarHighlight` | 10 x `Ptr<CTexture>` | the bar textures, copied from the entry's `CSimpleHudBarParams` by `Initialize` in the same order (**medium**; the list matches `shbp_...` of `hud-classes-params.md` section 9 name for name) |
| `hshe_fBarValue1`, `hshe_fBarValue1Max` | `FLOAT` | the first bar: value and maximum (**medium**: written by every `PrepareRenderingParameters`, read by the bar drawing) |
| `hshe_fBarValue2`, `hshe_fBarValue2Max` | `FLOAT` | the second bar (health and armour use it for the upper half; others leave it) |
| `hshe_strText`, `hshe_strLastText` | `CString` | the text to show and the text of the previous frame (a difference starts the shake) |
| `hshe_tmLastChange` | `Time` | when the text last changed (set to -100 s by `Reset`) |
| `hshe_fShakeOffsetX`, `hshe_fShakeOffsetY` | `FLOAT` | the current shake offsets of the text |
| `hshe_bResetInCurrentFrame` | `BOOL` | set by `Reset`, cleared by the next `RenderText`: a reset does not start a shake |
| `hshe_pcshepCommonParams` | pointer to `CCommonSimpleHudElmParams` | the four shake numbers of the database, fetched by `UpdateTemps` |

| Method (slot) | Basis | One-sentence purpose |
|---|---|---|
| `UpdateTemps()` (4) | R | call the base, then fetch the shared shake numbers from the database |
| `Initialize(Handle<CHUD>)` (6) | R | store the HUD, fetch parameters, copy the render-on-camera flag and the ten bar textures from the entry's bar parameters |
| `Reset()` (7) | R | clear both texts, set the last change to -100 s and mark "reset in this frame" |
| `PrepareRenderingParameters()` (8) | R | save the current text as the previous text (the element's override then fills the new one) |
| `Render(Vector2f *, float *)` (10) | R | the stacking rule of `hud-classes.md` section 8: apply the offset, draw bar, icon and text, return 1 |
| `GetIconTextureOverride()` (11) | R | default: no override |
| `GetIconStretch(icon cfg)` (12) | R | default: the configuration's stretch (`hic_fStretch`) |
| `GetTextStretch()` (13) | R | default: the constant 1 (**medium**) |
| `RenderBar(bar cfg, x, y)` (14) | P | draw one or two bars from the ten textures at the configured place; returns the height used |
| `RenderBarPart(...)` (15) | P | draw one segment (holder, fill, corner or highlight) of a bar |
| `RenderIcon(icon cfg, x, y)` (16) | R | take the override or the configured texture, scale it by the stretch and the HUD factor, place it at anchor plus offset, return the height |
| `RenderText(text cfg, x, y)` (17) | R | start the shake when the text changed (unless a reset is pending), compute the two shake offsets from the common numbers, set the bar parameters' font, place the text according to the alignment (0 left, 1 centre by half the width, 2 right by the whole width) and return the height |
| `AdjustColorForPhase1/2(color *)` (18, 19) | R | empty defaults; `CBossSimpleHudElement` overrides phase 1 |

### 3. The simple elements (25 classes)

"Params" is the data class of the entry (`hud-classes-params.md` section 4). "Wants" = the argument the element gives to `ShouldBeVisible`. All 25 override `PrepareRenderingParameters` (the seven power-ups through their common parent); the table lists what each reads and shows.

| Class (data name of the params) | Members of its own | Shows (confidence) |
|---|---|---|
| `CHealthSimpleHudElement` (`SHealthSimpleHudElementParams`) | none | the player's health as the text, and **two bars each covering half of the maximum health** (bar 1 the lower half, bar 2 the upper half); **wants** unless the player is in the riding state 3 (**medium**: the riding test is read; the clamps are **low**, the integer conversions were not recoverable) |
| `CArmorSimpleHudElement` (`SArmorSimpleHudElementParams`) | none | the armour as the text and the same two half-bars on the maximum armour; **wants** only when armour is above 0 and the player is not in riding state 3 (**medium**) |
| `CAirSimpleHudElement` (`SAirSimpleHudElementParams`) | none | the remaining breath as one bar (maximum 1): visible while the player puppet is in pose 3 and has been diving for a time that is below the air capacity of its puppet parameters; the fill is 1 minus the elapsed fraction (**low**: the pose value 3 and the capacity member were matched by offset only; the pose names are not in the data) |
| `CAdrenalineSimpleHudElement` (`SAdrenalineSimpleHudElementParams`) | none | **a stub**: sets bar 1 to 50 of 100 and **never wants to be shown** (**medium**: read in full, 16 lines) |
| `CFrozenSimpleHudElement` (`SFrozenSimpleHudElementParams`) | none | the player's `en_fFreezingRatio` as bar 1 of maximum 1; wants when the ratio is above 0 (**medium**) |
| `CBossSimpleHudElement` (`SBossSimpleHudElementParams`) | none | the **boss** of the world info (`en_henBoss`): bar 1 = the boss's health, maximum = its maximum health (through two virtual getters of the boss entity), text = the world info's `en_strBossName` translated **unless it equals `cecil`** (then empty), icon = the boss's own texture (`GetIconTextureOverride`); wants while the boss reports active and has health left; **phase 1** (a flag of the boss) flashes the bar colour between `0xFFDFDFDF` (the holder colour of the entry) and `0xFF7F7F7F` according to a sine of the time (`AdjustColorForPhase1`, read; the sign test was partly lost, **low** for the exact switching) (**medium**) |
| `CTimeSimpleHudElement` (`STimeSimpleHudElementParams`) | none | the world info's countdown: hidden while `en_fTimerDuration` is -1; otherwise bar 1 = duration minus the passed time (`en_fTimerPassedTime`), maximum = duration, and the remaining time as the text (**medium** for the bar, **low** for the text format) |
| `CCounterSimpleHudElement` (`SCounterSimpleHudElementParams`) | none | the world info's counter: hidden while `en_slCounterMax` is -1; otherwise bar maximum = `en_slCounterMax`, bar value = `en_slCounterValue` and the count as the text (**medium** for the bar, **low** for the text) |
| `CComboWeaponsSimpleHudElement` (`SComboWeaponsSimpleHudElementParams`) | none | the combo-weapons power: wants while the puppet reports combo weapons active and `en_fComboWeaponsTotalDuration` is above 0; bar maximum = that duration, bar value = duration minus the value of a virtual getter of the puppet (whether the getter returns the elapsed or the remaining time was not resolved), and a text built from it (**medium** for the visibility and the maximum, **low** for the rest) |
| `CLivesSimpleHudElement` (`SLivesSimpleHudElementParams`) | none | the number of lives as the text; wants only in a game that uses lives, is not a deathmatch and is not showing frozen statistics; until the **stretching delay** after `en_tmLifeAdded` has passed the shown number is one lower than the real count (**medium**); overrides `GetIconStretch` to **pulse the icon** (`hud-classes-params.md` section 6) |
| `CPointsSimpleHudElement` (`SPointsSimpleHudElementParams`) | none | the score from the game statistics (`0` when there are none); not in a deathmatch or with frozen statistics; overrides `GetTextStretch` to **pulse the text** after a life was added by score (`en_bLifeAddedByScore`) (**medium**) |
| `CBombsSimpleHudElement` (`SBombsSimpleHudElementParams`) | none | the number of **hand grenades** (`AI_HAND_GRENADES`) as the text; wants when there is at least one, the game info flag read is clear and the player is **not riding** (**medium**) |
| `CVehicleSimpleHudElement` (`SVehicleSimpleHudElementParams`) | none | while the player rides (state 3) a vehicle whose flags member has bit 0 clear (the member was matched by offset to `en_pnfNetFlags`, **low**): bar 1 = the vehicle's health of its maximum, the health as text; the icon is the vehicle's own (`GetIconTextureOverride`) (**medium**) |
| `CVehicleAmmoSimpleHudElement` (`CVehicleAmmoSimpleHudElementParams`) | none | while riding (state 3) a vehicle that has primary ammunition (`en_ctAmmoPrimary` not -1): that count as the text; icon from the vehicle (**medium**) |
| `COnlineNotificationHudElement` (`COnlineNotificationHudElementParams`; base of the params is `SSimpleHudElementParams`) | `onhep_uwIconSize` is on the params | online messages: hidden when the game is offline (a pending notification is dropped); online, it takes the next pending notification when none is showing, wants to be shown for a fixed time (a constant) and then removes the notification; a debug flag (probably the variable `hud_bFakeNotification` of the DLL's strings, **low**) forces it visible (**medium** for the cycle, **low** for the flag; the drawing is in its own `Render`) |
| `CPowerUpHudElement` (abstract; params base `SSimpleHudElementParams`) | `puhe_puiPowerUp` (`PowerUpIndex`) | for the power-up `puhe_puiPowerUp`: wants while the player's `IsPowerUpActive` is true; bar maximum = the power-up's duration (`samGetPowerUpDuration`), bar value = duration minus the time since it started (the player keeps one start time per power-up), the remaining seconds as text (**medium**) |
| `CSeriousStrengthHudElement`, `CSeriousSpeedHudElement`, `CSeriousJumpHudElement`, `CSeriousDamageHudElement`, `CSeriousScoreHudElement`, `CInvulnerabilityHudElement`, `CInvisibilityHudElement` | none | the seven power-ups: their constructors store `PowerUpIndex` **0 to 6** in this order (**high**: constructors read, order equal to `PUI_SERIOUS_STRENGTH`, `_SPEED`, `_JUMP`, `_DAMAGE`, `_SCORE`, `PUI_INVULNERABILITY`, `_INVISIBILITY` of `players-session-classes.md`); they use the parent's `PrepareRenderingParameters` and are the seven entries stacked at (4, 100) |
| `CAmmoHudElement` (abstract; params `SAmmoHudElementParams`) | none | the **ammunition of the weapon in one hand**: the weapon in the hand `GetPlayerHand()` of the player; wants when that hand holds a weapon, and the **left** hand's element shows only when its weapon differs from the right hand's weapon **and** uses a different ammunition type; its `Render` draws the ammunition icon `samGetAmmoIcon(samGetAmmoForWeapon(weapon))` (when `ahep_bShowIcon`) and the quantity (when `ahep_bShowText`) at the icon and text places of `SAmmoHudElementParams` (**medium**) |
| `CLeftAmmoHudElement`, `CRightAmmoHudElement` | none | `GetPlayerHand` returns **0** (left) and **1** (right); everything else is inherited (**high**: the two functions are 7 lines each) |

The **unique members** of these classes are only `puhe_puiPowerUp`; every other simple element adds **no members**: its `S`/`C...Params` entry carries the numbers (`hud-classes-params.md` section 4) and its class only the formulas of this table.

### 4. Exported methods of the 28 classes

| Class | Named methods (basis, virtual slot) |
|---|---|
| `CHudElement` | `Initialize` (R, 6), `UpdateTemps` (R, 4), `ShouldBeVisible` (R, 9), `OnHudCommand` (R, 5), `PrepareRenderingParameters` (R, 8), `Reset` (R, 7), `Render` (R, 10) |
| `CSimpleHudElement` | `UpdateTemps`, `Initialize`, `Reset`, `PrepareRenderingParameters`, `Render` (the slots above), `GetIconStretch` (R, 12), `GetTextStretch` (R, 13), `GetIconTextureOverride` (R, 11), `AdjustColorForPhase1` (R, 18), `AdjustColorForPhase2` (R, 19), `RenderBar` (P, 14), `RenderBarPart` (P, 15), `RenderIcon` (R, 16), `RenderText` (R, 17) |
| the 14 simple elements | one `PrepareRenderingParameters` each (R, 8); in addition `CBossSimpleHudElement` has `GetIconTextureOverride` (R, 11) and `AdjustColorForPhase1` (R, 18); `CLivesSimpleHudElement` has `GetIconStretch` (R, 12); `CPointsSimpleHudElement` has `GetTextStretch` (R, 13); `CVehicleSimpleHudElement` and `CVehicleAmmoSimpleHudElement` have `GetIconTextureOverride` (R, 11) (20 methods in all) |
| `CPowerUpHudElement` and its seven children | `PrepareRenderingParameters` on the base only (R, 8); the children export no method |
| `CAmmoHudElement` | `GetPlayerHand` (R, 20), `PrepareRenderingParameters` (R, 8), `Render` (P, 10) |
| `CLeftAmmoHudElement`, `CRightAmmoHudElement` | `GetPlayerHand` (R, 20) |
| `COnlineNotificationHudElement` | `Initialize` (R, 6), `PrepareRenderingParameters` (R, 8), `Render` (N, 10) (3 methods) |

133 named methods exist on the 46 element classes; the 28 classes of this page export **50** of them (7 + 14 + 20 + 1 + 3 + 1 + 1 + 3; the other 83 are on the 18 other elements, `hud-classes-special.md`). Every class additionally has the seven standard symbols.

## Invariants

1. Every simple element class has the **20 slots** of `CSimpleHudElement` (the ammunition elements 21) and overrides slot 8; the seven power-up children share slot 8 with `CPowerUpHudElement` (script-independent: the 21 virtual tables were compared; 28 of 28).
2. The `PowerUpIndex` of the seven children is 0 to 6 in the order above; the two hand variants differ **only** in `GetPlayerHand` (0 and 1).
3. A simple element's visibility wish is a boolean; the final visibility is `ShouldBeVisible(wish)` (`hud-classes.md` section 7).
4. A simple element whose parameters have no text, icon or bar configuration (null handle) draws that part as nothing and uses 0 as its height (`RenderText` and `RenderIcon` return 0 for a null configuration).

## Open questions

1. The **exact formulas** of the bars and texts of health, armour, air, time, counter and combo weapons (the integer conversions, the clamps): T01-187.
2. The drawing of the bar (`RenderBar`, `RenderBarPart`): which texture goes where, the two-bar layout, the phase gradients: T01-187.
3. The **air** element's pose value 3 and capacity (the pose names are not in the data): T01-187.
4. `COnlineNotificationHudElement::Render`, the notification time and the debug flag: T01-191.
5. Why `CAdrenalineSimpleHudElement` is a stub (a leftover of an unused mechanic): not answerable from the binary; marked here.
6. The boss element's two virtual getters and the boss icon member: T01-189 (boss entities, `bosses.md`).

## Test vectors

1. **Power-up elements.** `CSeriousJumpHudElement` has `puhe_puiPowerUp` = 2 (`PUI_SERIOUS_JUMP`); with that power-up active for 5 s of a 30 s duration it wants to be shown with bar 1 = 25 of 30 and the text for 25 seconds.
2. **Ammunition.** With a single weapon in both hands (the left and right weapon equal) the left element's wish is false and the right element's is true.
3. **Lives.** With 3 lives and a life added 1 s ago (delay 1.5 s): the shown number is 2; at 1.5 s it becomes 3 and the icon pulses until 2.2 s (period 0.7 s, extra stretch 0.6).
4. **Boss.** With `en_strBossName` = `cecil` the boss bar is drawn without a name; with any other name the translated name is the text.
