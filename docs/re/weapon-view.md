# Weapon view and rendering parameters (T01-121)

- **Roadmap task:** T01-121 (follow-up of `weapon-classes.md` open question 5; with `camera-view.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/weapon_view_verify.py` reproduces from the data: the six sway members and the double-weapon fire delay of the 17 weapons (single player and deathmatch), the eight rendering members of the 16 rendering files (with the field of view in degrees), the sniper zoom numbers and the multiplier curve they give, the player's view-spring numbers; **medium** for everything read from `Sam2Game.dll` (which member does what, the formulas, the order of the placement, the double-weapon fire rule, the aspect-ratio correction, the zoom presentation, the developer adjust mode), because each rests on **one reading** of the decompiled functions plus a read of the numeric constants and **nothing was run**; **low** for the items marked low and for "Open questions".

Prerequisites: `weapon-classes.md` (sections 4.1 and 4.3 name the members), `camera-view.md` (the data of the view), `hitscan-shot.md` (section 3 uses the weapon-in-view matrix for the muzzle), `projectile-launch.md` (charging), `players-session-classes.md` (the player puppet).

## Summary

The first-person weapon is drawn **after** the world with its own projection and a squeezed depth range, at a placement that is the owner's view placement times a small chain of offsets. The members that `weapon-classes.md` could only name now have their roles:

1. **`bwp_fOffsetX`, `bwp_fOffsetY`, `bwp_fOffsetZ`, `bwp_fFrequencyX`, `bwp_fFrequencyY`, `bwp_fOffsetZExponent` are not walking sway.** They are the **charge and recoil displacement** of the weapon model, and **only weapons with the charging flag use them**: the grenade launcher, the cannon and the zap gun (section 2). The other 14 files carry the same six members and the engine never reads them (the data has 0.01, 0.01, 0.4, 10, 8, 2 in 14 files and 0.005, 0.005, 0.4 (zap gun 0.1), 5, 6, 3 in the three).
2. **Walking bob is not in the data** (`camera-view.md` said so) but it is in code: a **triangle wave of frequency 7/s** and a strafe lean, scaled by `bwrp_fBobbingMul` and by 1.0 to 1.5 while sprinting (section 3).
3. **Two weapons (one per hand):** the second set of rendering members (`bwrp_vPositionDoubleWeapon`, `bwrp_vAngleDoubleWeapon`, `bwrp_fFOVDoubleWeapon`) is used when **both hands hold a weapon**; the left weapon is drawn **mirrored in X**. `bwp_fDoubleWeaponFireDelay` is the delay with which two **equal** weapons alternate their shots (section 5).
4. **`bwrp_fFOV` is a horizontal field of view in radians** (0.53 to 1.23 = 30.4 to 70.4 degrees) for a 4:3 screen; the weapon projection is built from it with **`bwrp_fFrontClipDist` as the near plane (0.1) and 10,000 as the far plane**, and the weapon is drawn in the **first 10 percent of the depth range** so that it never sinks into walls (section 4).
5. **The main view's field of view** (`camera-view.md` open question): 80 degrees **horizontal at 4:3**, widened for other aspect ratios by `2 atan(aspect × 0.75 × tan(fov / 2))`, limited to 45 to 135 degrees, then **multiplied by the player's field-of-view multiplier**, which is how the **sniper zoom** works (section 6).
6. **The developer adjust mode** edits the rendering members live (section 7).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Exported functions read: `CBaseWeaponEntity::GetWeaponInViewMatrix`, `GetWeaponChargeDisplaceMatrix`, `GetWeaponAbsPlacement`, `Render`, `AddRenderingCommand`, `GetAbsPlacement`; `CPlayerPuppetEntity::GetFOVMultiplier`, `SetWeaponFOVMultiplier`, `AnimateViewOrigin`, `UpdateBobbingValues`, `UpdateDampingValues`, `UpdateSwimWobbling`, `UpdateViewOriginInterpolator`, `InjectRenderingCommands`, `RenderView`, `CanFireFromWeaponNow`, `GetWeaponInActiveHand`; `CPuppetEntity::GetProjectionMatrix`; `CSniperWeaponEntity::OnCreate`, `OnStep`, `Render`, `ActivateZoomMode`, `DeactivateZoomMode`, `OnAlternativeFirePressed`, `PutDown`, `IsZooming`; `CAdjustWeaponMode::BindCommands`, `EvaluateControls`, `RenderView`, `UnBindCommands`; the default constructors of the weapon entities. Member roles were assigned by the byte positions of the members in the weapon parameter object matched against the order rule of `weapon-classes.md` section 4. The constants were read from the data section. Time is a 64-bit count with 2^32 ticks per second.
- **Data.** `tools/re/weapon_view_verify.py [DATA_DIR]` (about 1 minute) prints the tables of sections 1 to 6 below.
- **Not checked:** nothing was run; the player's whole view animation (`AnimateViewOrigin` was read only for its structure: swim wobble, a banking roll and the added height), the camera of third person and of vehicles (`camera-view.md`), the rendering of the model itself (`mdl.md`), the dedicated "zoom" sound event, and the setting that switches the aspect-ratio correction on.

## Layout / rules

### 1. The members and the objects that hold them (**medium**)

The weapon entity holds its parameter object and its rendering parameter object. In the rendering object the eight members are, in this order, **position (3 floats, metres in the view's own frame: x right, y up, z forward is negative), angle (3 floats, degrees: the first two are the turns, the third is 0 in all files), field of view (radians), bobbing multiplier, then the same position, angle and field of view for two weapons, and the near distance**. The 16 files are listed by the script (section 2 of its output); ranges: position x 0.09 to 0.28, y -0.59 to -0.16, z -0.77 to 0.08; angle -7.1 to 24 degrees; FOV 0.531 (zap gun) to 1.228 (beam gun); bobbing multiplier 0.4 to 1.0; near distance **0.1 in all 16**. Two-weapon values equal the one-weapon values in 3 of 16 files (Colt, Uzi, Serious Bomb); otherwise only the x position differs (up to +0.133), except the rocket launcher whose two-weapon angle also differs (0.68, 2.18 against 3.10, 2.13).

### 2. Charge and recoil displacement (`GetWeaponChargeDisplaceMatrix`, **medium**)

Only for weapons whose **charging flag** is set (grenade launcher, cannon, zap gun); every other weapon returns "no displacement". Let `t` = seconds since the last **charge start** (`DoCharging`, not reset by a shot), `C = clamp(t / bwp_fChargingTime, 0, 1)` and `R = clamp(t / recoil duration, 0, 1)`.

- **Charging state:** `k = 1`, `s = C`.
- **Recoil state:** `k = 1 - R`, `s = 0`.
- **Any other state:** no displacement.
- Translation (no rotation): **x = sin(2π × bwp_fFrequencyX × t) × bwp_fOffsetX × s × k**; **y = sin(2π × bwp_fFrequencyY × t) × bwp_fOffsetY × s × k**; **z = (1 - e^(-t × bwp_fOffsetZExponent)) × bwp_fOffsetZ × k** (the base is Euler's number, read as 2.71828).

So while a charge builds, the weapon **shakes in x and y with growing amplitude** (`s` grows from 0 to 1 over the charging time) and **sinks away along z towards `bwp_fOffsetZ`** with a time constant `1 / bwp_fOffsetZExponent` (0.33 s for the grenade launcher and zap gun, 0.5 s for the cannon); in the recoil state the shake stops and the z offset fades with `k`. Because `t` counts from the charge start, a recoil that follows a charge longer than the recoil duration has `R = 1` and **no visible kick** (the cannon, whose charge takes 1.25 s against a recoil of 0.5 to 0.75 s) (**low**: read, but unusual; it may be a defect of the original).

| Weapon | OffsetX, Y | OffsetZ | FrequencyX, Y | Exponent |
|---|---|---|---|---|
| Grenade launcher | 0.005, 0.005 | 0.4 | 5, 6 | 3 |
| Cannon | 0.01, 0.01 | 0.4 | 10, 8 | 2 |
| Zap gun | 0.005, 0.005 | 0.1 | 5, 6 | 3 |

### 3. The weapon placement in the world (`GetWeaponInViewMatrix`, `GetWeaponAbsPlacement`, **medium**; order **low**)

1. **Weapon in view** = rotation by the file's angle (degrees converted to radians, turns as heading and pitch) and translation by the file's position, taken from the **one-weapon** or the **two-weapon** triple (two-weapon when the player has **both** a left-hand and a right-hand weapon object). For a **left-hand** weapon (`en_bRightHandWeapon` false) the whole matrix is **mirrored in X**, and the model is drawn with a **negated X stretch** (undone after drawing).
2. **Bob and lag** (`GetWeaponAbsPlacement`), added to the weapon-in-view placement, are built from the player's runtime values: a **walk-bob phase** `p` in [-1, 1] (a triangle wave: it moves at **7 per second** towards +1 or -1 and flips at the ends while the player's relative-move vector has a non-zero X or Z component; with no movement it returns to 0 at 2 per second), a **strafe lean** `q` (moves at 8 per second towards +1 for one strafe direction and -1 for the other, returns to 0 at 8 per second) and a **sprint factor** `f`: 1.0 when walking, rising to 1.5 over 0.5 s after sprint starts, and falling back from 1.5 to 1.0 over 0.5 s after it stops. With `b = bwrp_fBobbingMul`: `u = b × p × f × (-0.02)` (metres), the weapon is moved by **`u` along its own X** and by **`30 × u²` along its own Y** (a small U-shaped path of at most 0.03 m by 0.027 m). The strafe lean `q` does **not** move the weapon: it feeds the view roll (`AnimateViewOrigin`, T01-224).
3. **View damping.** The player keeps two damped springs (horizontal and vertical offsets of the view) updated every step with stiffness `ppp_fViewDampingK` = 160 and damping `ppp_fViewDampingD` = 10 (all four player files), each driven by the player's vertical velocity change of the step and limited to the range -0.4 to +0.1 (-0.1 to +0.1 while swimming); the vertical one, times **0.15**, is added to the weapon's height (the weapon dips and lags when the player lands).
4. **Charge displacement** of section 2 is applied last, in the weapon's own frame.

(The exact multiplication order was read from a long unrolled function; the sequence "view, bob, weapon in view, charge displacement" is **low**; the terms above are what each part contributes.)

### 4. Drawing the weapon (`CBaseWeaponEntity::Render`, **medium**)

1. Only for a player owner; the weapon is drawn in a **second pass after the world**.
2. **Field of view:** `fov = bwrp_fFOV` (or `bwrp_fFOVDoubleWeapon` with two weapons), in radians. When the screen aspect is not 4:3 **and** the engine's weapon-aspect switch (a runtime setting, initially on in the data section) is set, `fov' = (360 / π) × atan(aspect × 0.75 × tan(π × fov / 360))`; because the formula treats the value as degrees but the value is a few tenths of a radian, **this is, to a precision of 1e-4, a plain multiplication by `aspect / (4/3)`**: at 16:9 the zap gun's 0.5311 becomes 0.7081 (**medium**; one reading).
3. The projection is `mthFrustumFOVX(fov', aspect, near = bwrp_fFrontClipDist, far = 10000)` (a horizontal-FOV frustum; the same function the main view uses with radians), with the view matrix of the weapon placement and the **depth range limited to 0 to 0.1**; the previous projection and depth range are restored afterwards.
4. The model is drawn at the weapon placement (section 3), mirrored for the left hand.
5. The sniper in **zoom** draws no weapon model (section 6).

### 5. Two weapons and `bwp_fDoubleWeaponFireDelay` (`CanFireFromWeaponNow`, **medium**)

A weapon may start its shot only when its state is **idle or boring** and (rule in this order):

1. In **combo-weapons mode** (the game's combo option is on **and** the player has the combo mode active) the answer is **yes**.
2. Let the partner be the weapon in the other hand. With **no partner, or a different weapon index**: **yes** (no delay).
3. **Equal weapons** (same index in both hands), with `d` = this weapon's `bwp_fDoubleWeaponFireDelay`:
   - If the partner is **reloading**: **yes** when the time since the **later** of the two last shots is at least `d`.
   - If this weapon fired **more recently** than the partner: **no** (the partner is due).
   - If this is a **left-hand** weapon and **neither** weapon has ever fired (both last-fire times are still the initial value -100 s): **no** (the right hand fires first).
   - Otherwise **yes** when the time since the **partner's** last shot is at least `d`.

So two equal weapons **alternate**, the right hand first, each firing `d` seconds after the other. Values `d` (single player; deathmatch the same except the Uzi 0.1): saw, shotguns, sniper, rocket launcher, Serious Bomb, Klodovik, auto shotgun and beam gun 0.2; Colt 0.25; Uzi 0.08; minigun and flamer 0.05; plasma rifle 0.075; grenade launcher 0.5; cannon 0.75; zap gun 0.2. The fire button mapping (the left weapon uses the second button, or the first in combo mode) is `players-session-classes.md`.

### 6. The sniper's zoom presentation (**medium**)

- **Toggle.** The alternative fire button toggles: the first press **activates**, the next **deactivates**; putting the weapon down also deactivates. Activating records the **start time**, **forces the first-person view** (the owner's third-person flag is saved and cleared; deactivating restores it) and sets the shooter's zoom damage (`hitscan-shot.md` 2.5); deactivating sets the owner's **field-of-view multiplier back to 1.0**.
- **Zoom curve.** At creation: `1 / swp_fMinZoomFactor`, `1 / swp_fMaxZoomFactor` and `swp_fFullZoomingTime` are stored. Each step while zoomed (right-hand weapon): `r = clamp((now - start) / fullZoomingTime, 0, 1)`; **multiplier = lerp(1 / min, 1 / max, r)**, written to the owner (`SetWeaponFOVMultiplier`). The main view multiplies its field of view by it: single player 0.5 at the start, 0.25 after 0.8 s, i.e. **80 degrees → 40 → 20** (deathmatch: 0.5 to 1/6 over 1.0 s, 40 → 13.3 degrees); a sound is started once when the zoom has reached 1.0 within 0.01 (**low**: its role was not read). `IsZooming` reports the flag; the crosshair is hidden while zoomed.
- **Overlay** (drawn instead of the weapon model while zoomed, in a **640 × 480 virtual screen**, medium-low): the mask texture `SniperMask` as four mirrored quarters centred at (200, 120), (440, 120), (200, 360), (440, 360), a rotated `SniperWheel` at (320, 240), black bars over x 0 to 80 and 560 to 640, a cross-hair of two grey lines (a vertical one at x 320 and a horizontal one at y 240), the `SniperEye` and `SniperArrow` pieces at (480, 230) and (160, 230), the `SniperLed` at (285, 275), **green when the weapon is idle or boring and red otherwise**, the zoom factor as text right-aligned at x = 488, y = 245 and the **distance to the aimed point** (`"---.-"` when none) at x = 152, y = 245, coloured with the aimed entity's alignment colour. The numbers are the code's constants; the textures are game content and not described further.

### 7. The developer adjust mode (`CAdjustWeaponMode`, **medium**)

A tool for tuning the rendering members (`weapon-classes.md` section 8), bound to the keyboard and mouse: **F9** enables it ("Enable weapon adjusting"); the on-screen help reads *Move LRUD: Ctrl+LMB; Move In-Out: Ctrl+RMB; Rotate: Ctrl+LMB+RMB; Adjust FOV: Ctrl+Shift+LMB*. While active it **edits the rendering object of the right-hand weapon in place** (and marks it changed so that the view follows): the one-weapon members, or the **two-weapon members when the player holds two weapons**. Mouse movement is scaled by 0.01 per unit for the FOV and the moves (rotation uses its own constant); the command "Copy single to double weapon params" copies the single position, angle and FOV into the double members. It is not part of normal play; the retail data was tuned with it (the 16 files hold its results), and nothing in the data enables it.

## Invariants

1. Every rendering file has the eight members; `bwrp_fFrontClipDist` is 0.1 and the third angle is 0 in 16 of 16.
2. The six sway members are the same in 14 of the 17 files and differ only in the three charging weapons' files.
3. `bwp_fChargingTime` is above 0 exactly for the three weapons that use the six sway members.
4. Left-hand weapons are mirrored in X (no file stores a left-specific placement).
5. The horizontal FOV is 80 degrees at 4:3 and 45 to 135 degrees after widening and zoom; the weapon's own FOV is 30 to 71 degrees.

## Open questions

1. **The view animation of the player** (`AnimateViewOrigin`: swim wobble amplitude 0.025 with three phases of 6.98 s, 3.70 s and 2.51 s, the banking roll from move and side-step, the added height) and the third-person cameras: only the structure was read. **T01-224.**
2. **Where the weapon-aspect switch is set** and whether the weapon FOV correction can be off; whether the rule of section 2 for a recoil after a long charge is a defect: **T01-124** (run-time).
3. The exact multiplication order of the placement chain (section 3) and the rotation of the sniper wheel: **T01-224.**
4. The **left-hand weapon's** use of the fire buttons and the creation of the second weapon object (`players-session-classes.md`, T01-114).

## Test vectors

Derived from the data and the formulas above; **not** run on the original.

1. **Grenade launcher charge shake.** At t = 0.25 s of a charge (charging time 0.5): `s = 0.5`; x = sin(2π × 5 × 0.25) × 0.005 × 0.5 = sin(7.854 rad) = 1.0 × 0.0025 = **0.0025 m**; y = sin(2π × 6 × 0.25) = sin(9.425) = 0 → **0**; z = (1 - e^(-0.25 × 3)) × 0.4 = (1 - 0.4724) × 0.4 = **0.211 m**.
2. **Cannon, 1.0 s into the charge** (charging time 1.25): `s = 0.8`; x = sin(2π × 10 × 1.0) = 0 → 0; z = (1 - e^(-2)) × 0.4 = **0.346 m**.
3. **Bob.** Walking, bobbing multiplier 0.9 (grenade launcher), sprint factor 1.0, phase 1: `u = 0.9 × 1 × 1 × -0.02` = -0.018: the weapon moves 0.018 m to the left and 30 × 0.018² = **0.0097 m** up. Sprinting for 0.5 s or more: `f = 1.5`, `u` = -0.027, up 0.0219. The phase goes from -1 to +1 in 2/7 = 0.286 s (period 0.571 s).
4. **Aspect.** Sniper FOV 0.5706 at 16:9: 0.5706 × (16/9) / (4/3) = **0.7608** (formula result equal to 4 digits). At 4:3: unchanged.
5. **Zoom.** Single player: 0 s → multiplier 0.5 (40 degrees); 0.4 s → 0.375 (30 degrees); 0.8 s and later → 0.25 (20 degrees); toggled off → 1.0 (80 degrees). Deathmatch: 1.0 s → 1/6 (13.3 degrees).
6. **Two equal weapons** (Uzi in both hands, `d` = 0.08 single player): neither has fired: only the right hand may fire; at t = 0 right fires; the left may fire from t = 0.08; the right again from the left's shot + 0.08 = 0.16; and so on, a shot every 0.08 s in turn. Colt: every 0.25 s.
7. **Two-weapon placement.** The auto shotgun alone: position (0.1623, -0.2186, -0.3995); with two: (0.2455, -0.2189, -0.3995); the left one is mirrored in X; both with angle (6.61, 5.59, 0) and FOV 0.8262 rad (47.3 degrees).
