# Menu classes: popups, underlays and the loading screen (T01-112, page 6 of 6)

- **Roadmap task:** T01-112 (overview: [`menu-classes.md`](menu-classes.md); this page covers the 17 classes of the "popups" group)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, the popup layouts (gadget names, underlays) and the answer names (script); **medium** for the answer protocol and the PleaseWait and timed popups (read; one reading each, bodies small and consistent with the callers on every page); **low** for the timer details of `StepTimed` and for the loading screen's level data.

Prerequisites: [`menu-classes.md`](menu-classes.md) sections 2 and 4 (base interface, popup protocol), `gui.md`.

## Summary

Seventeen classes: **13 popup screens** (`CPopupMenuYesNo`, `YesNoTimed`, `Ok`, `Cancel`, `AbortGame`, `EnterText`, `PleaseWait` and the five `...Large` variants), **three underlay screens** (`CMSPopupMenuUnderlay`, `...Large`, `...Text`) and the **loading screen** `CMSLoading`. **172 exported functions** (119 standard + 53 named), 9 classes with named methods; the `Large` variants (5) and the three underlays (two of them) have none: they are **pure layouts** that reuse the code of their small twin through the layout's screen class name (`menu-classes.md` section 3: `CPopupMenuAbortGameLarge` is run by the class named `CPopupMenuOkLarge`).

## Evidence

Export names and layouts (script section 6); the bodies of all popup methods were decompiled (they are 5 to 438 bytes): `YesNo`, `Ok`, `Cancel`, `EnterText`, `PleaseWait` (all five methods), `YesNoTimed`, `CMSLoading` (`GetLevelData`, `SetupLoadingScreen`, `SetImage`, `SetLevel`, `IsInteractive`). The machine code of the `OnCommandPressed` and `PostCreate` functions of the popups is **one shared function each**, so the exported names of five classes resolve to the same body. **Not checked:** the level information record behind `GetLevelData`, the loading model and the scrolling description.

## Layout / rules

### 1. The popup layouts (data, script section 6)

Every popup layout has the label **`lblMessageText`** (the message) and its answer gadgets; its underlay screen is one of the three underlays. Class and gadgets:

| Layout stem | Class named in the layout | Underlay | Gadgets |
|---|---|---|---|
| `cpopupmenuyesno`, `...yesnolarge`, `...yesnotimed` | `CPopupMenuYesNo`, `CPopupMenuYesNoLarge`, `CPopupMenuYesNoTimed` | `CMSPopupMenuUnderlay` (Large: `...Large`) | `lblMessageText`, `btnYes`, `btnNo` |
| `cpopupmenuok`, `...oklarge`, `...abortgame`, `...abortgamelarge` | `SPopupMenuOk` (two layouts), `CPopupMenuOkLarge` (two layouts) | small / large | `lblMessageText`, `btnOk` |
| `cpopupmenucancel`, `...cancellarge` | `SPopupMenuCancel`, `CPopupMenuCancelLarge` | small / large | `lblMessageText`, `btnCancel` |
| `cpopupmenuentertext`, `...entertextlarge` | `CPopupMenuEnterText`, `CPopupMenuEnterTextLarge` | `CMSPopupMenuUnderlayText` / large | `lblMessageText`, `etEnterText` (the text entry), `bbrButtonBar` |
| `cpopupmenupleasewait`, `...pleasewaitlarge` | `CPopupMenuPleaseWait`, `CPopupMenuPleaseWaitLarge` | small / large | `lblMessageText`, `lblPleaseWait`, `pbProgressBar`; the small one also `imgImageRotating` and `imgSam2Logo` |
| `cmspopupmenuunderlay`, `...large`, `...text` | `SPopupMenuUnderlay` (all three) | `CMSUnderlay` | one gadget `bckBackground` (the in-game menu background) |

**The name `SPopupMenuYesNo` that the code passes when it opens a yes/no popup is not a layout stem or class in the data**: the layout is `cpopupmenuyesno` (class `CPopupMenuYesNo`). The caller strings `SPopupMenuYesNo`, `SPopupMenuOk` (and `CPopupMenuYesNoLarge`, `CPopupMenuOkLarge`, `CPopupMenuEnterText`, `CPopupMenuPleaseWait`) therefore name the popup by a **name that resolves to the class, the S twin or the stem** (the same case-folded "class first, then stem" resolution `gui.md` found for links; the S twin rule is `menu-classes.md` section 3). This is **medium**: the resolution function was not read (T01-197).

### 2. The answer protocol (read, medium)

A popup is opened by the base function of every screen (arguments: popup name, message name, text, and for yes/no the button to focus). The popup's methods answer by **raising a named event on itself**, which the base class delivers to the screen below as `<message>_<event>` (`menu-classes.md` section 4). Exact event names per class:

| Class | Method | Raises |
|---|---|---|
| `CPopupMenuYesNo` | `btnYes_OnClick`, `btnNo_OnClick` | `OnYes`, `OnNo` |
| `CPopupMenuOk` (and the layouts of `AbortGame`) | `btnOk_OnClick` | `OnOk` |
| `CPopupMenuCancel` | `btnCancel_OnClick` | `OnCancel` |
| `CPopupMenuEnterText` | `etEnterText_OnClick` | stores the typed text in the variable `EnteredText`, then `OnAccept` |
| | `etEnterText_OnCancel` | clears `EnteredText`, then `OnCancel` |
| `CPopupMenuPleaseWait` | `Step` | `OnSuccess`, `OnFailed`, or `FinishedLoading` (below) |
| `CPopupMenuYesNoTimed` | `Step` | `OnNo` when its time has passed |

The same four popups `YesNo`, `Ok`, `Cancel`, `PleaseWait` share **one `OnCommandPressed`**: **command 12 (back) returns 1 and does nothing** (swallowed: the popup cannot be dismissed by back), every other command goes to the base class. `OnSetCurrent` and `OnUnsetCurrent` are empty (5-byte) functions; `PostCreate` (shared with `CMSInGameMenu`, `CMSOnlineLogin` and `CMSXboxLivePassEntry`) calls the base `PostCreate` and then the virtual function at slot 2 with 0 (a base-class initialisation, name not known: T01-203). `CPopupMenuAbortGame::btnOk_OnClick` is the same body as `CMSLoading::SessionNotAvailable_OnOk` and `CMSInGameMenu::StopGame_OnSuccess`: **stop the session and leave the game** (three calls; read, low).

**`CPopupMenuEnterText`** also exports `SetEditText(text)`: it finds the gadget `etEnterText` and puts the text into it (used by the save, record-demo and session-name screens to preset a name). `CPopupMenuEnterTextLarge` has the same layout gadgets but **no methods of its own**: the stem `cpopupmenuentertextlarge` names the class `CPopupMenuEnterTextLarge`, which exports only standard symbols, so its answers cannot come from that class (open: T01-203).

### 3. `CPopupMenuYesNoTimed` (layout `cpopupmenuyesnotimed`; 9 functions, 2 named)

A yes/no popup that **answers `OnNo` by itself after 10 seconds**. `PostCreate` sets the limit to **10.0 s** (a float member), stores the **start time** (the engine's uptime clock) and focuses the base. `Step` each frame: elapsed = now minus start; **if the limit is smaller than the elapsed time it raises `OnNo`**; then it fills the label `lblMessageText` with `<message>\n<seconds left>` where the seconds left are the limit minus the elapsed time **clamped to 0 .. limit** (format `%1\n%2`; the message text is kept the first time). The only caller seen in the strings is the **video options keep-or-revert** question (`msgKeepSettings`, `menu-classes-options.md`), which fits "revert after ten seconds" (name evidence: the caller is not seen: T01-203). All of this: read, medium; the rounding of the seconds shown is not read.

### 4. `CPopupMenuPleaseWait` (layout `cpopupmenupleasewait`; 17 functions, 10 named) and `CMSLoading`

A **progress popup** with a bar `pbProgressBar` and a message. **Declared members** (the binary's table of the screen; names only): the progress type `ms_prtProgresstype`, a minimum and maximum duration `ms_fMinDuration`, `ms_fMaxDuration`, a start time `ms_tmStartTime`, the current state `ms_pfrCurrentState` (the callback's last result), a target time `ms_fTargetTime` and a finished flag `ms_bFinished`.

| Method | Behaviour | Basis |
|---|---|---|
| `StartTimedProgress(min, max, callback)` | type **0 (timed)**; the minimum is raised to at least **0.1 s**, the maximum to at least the minimum; stores the callback, the target = maximum, resets the finished flag and the bar to 0 | read |
| `StartPolledProgress(callback)` | type **1 (polled)**; stores the callback, resets the flag and the bar to 0 | read |
| `Step` | with no callback stored logs a guard breach once and does nothing; looks up the gadget `pbProgressBar` and, if it is missing, **logs `Progress bar is missing from the menu screen.` and raises `OnFailed`**; otherwise runs `StepTimed` (type 0) or `StepPolled` (type 1), writes the bar's value (the progress 0 to 1), and keeps the base `Step` | read |
| `StepPolled` | calls the callback every frame until it returns a result: **1 = success** raises `OnSuccess`; **3 = finished loading** raises the event `FinishedLoading` (delivered as `<message>_FinishedLoading`, which is how `CMSLoading::LoadLevel_FinishedLoading` is reached) and sets the bar to 1; any other non-zero result raises `OnFailed` and sets the bar to 1; **0 = still running** (the bar is left to the callback) | read, medium |
| `StepTimed` | the bar is elapsed time over a **target** between the minimum and maximum; the callback is polled until its first non-zero result; when the elapsed time reaches the target the popup raises `OnSuccess` if the last result was 1, else `OnFailed`; a quick success shortens the target to the minimum duration so that the popup is shown at least that long | read, **low** (branch details) |
| `IsInteractive` | returns 0: **the popup takes no input** (`CMSLoading` too) | read |

The callback returns a small enumeration (`EProgressFuncResult`: 0 running, 1 success, 3 finished loading, others failure). The callers pass either a plain function that does the work in steps (the online connect, the Gamespy connect) or a loader.

**`CMSLoading`** (layout `cmsloading`, 24 gadgets: the loading background `bckBackground`, `pbProgressBar`, `lblMessageText`, `lblLevelName`, `lblEpisodeName`, `lblModelName`, `lblContinue`, the render model `rmRenderModel` with `ibtnArrowUp` / `ibtnArrowDown` and the scrolling description `stDescription`, 13 images; 23 functions, 16 named). **It is a `CPopupMenuPleaseWait`** (its constructors call the popup's): it uses the same step machinery with the loader as polled callback.

| Method | Behaviour | Basis |
|---|---|---|
| `Step` | the popup `Step` and `GetLevelData` on the first frames | read |
| `GetLevelData` | asks the engine for the **path of the world being loaded** (empty: no level information); on a **client that is still connecting** (not in the editor) it opens a PleaseWait popup "Connecting to host" (message `ConnectingToHost`, 3-argument open), reads the variable `ConnectingWorldFile` (set by the connection) and takes that world's level information; then `SetupLoadingScreen`, sets the message name to `LoadLevel`, and starts the **polled** progress with the connecting callback when connecting, else the text "Loading..." with the loading callback | read, medium |
| `SetupLoadingScreen(info, found)` | default picture `Content/SeriousSam2/Textures/LoadingScreens/Default.tex`; with level information: if the info's picture path is not empty **and the file exists** it replaces the default; the info's two names (level and episode) go to `SetLevel`; without information both names are empty | read |
| `SetImage(path)` | sets the texture of the loading background gadget | read |
| `SetLevel(level, episode)` | writes `lblLevelName` and `lblEpisodeName` | read |
| `LoadLevel_FinishedLoading` (242 bytes) | when the load is done shows the label `lblContinue` with `ETRSLoadingFinished.MessagePC` ("Please press a key to continue") or `...MessageXbox` ("Please press Start or A to continue") according to the platform, so **the player must press a key before play starts** | profiled |
| `OnCommandPressed` | after loading, any command closes the screen and starts play; before that commands are ignored (`IsInteractive` returns 0 until then; 85 bytes) | profiled, low |
| `OnSetCurrent` (442 bytes) | shows `lblModelName` and the scrolling description of the level's **loading model** (the layout's render model gadget) | profiled |
| `ibtnArrowUp/Down_OnClick` | scroll `stDescription` | profiled |
| `ConnectingToHost_OnSuccess` / `_OnFailed` | the answers of the connection popup: success calls `SetupLoadingScreen` and starts the loader as polled progress (message `LoadLevel`), failure shows an OK popup "This game session is no longer available." and `SessionNotAvailable_OnOk` leaves the game | read / profiled |

Only the **loading screen** class is not a "popup" in the flow: it is opened by the engine when a level starts loading (not by a link: `menus-tables.md` lists `CMSLoading` as an entry point).

### 5. The underlays

`CMSUnderlay` (75 screens use it as their underlay), `CMSOverlay`, `CMSMainMenuUnderlay` are on the shell page. The three popup underlays are **layouts only** here: `CMSPopupMenuUnderlay` has a `PostCreate` of 8 bytes (an own function, not the shared stub) that I did not read; `...Large` and `...Text` export nothing. All three layouts carry one gadget `bckBackground` of the in-game menu background kind and use `CMSUnderlay` as their own underlay: **popup screens stack three layers** (popup gadgets, popup underlay, `CMSUnderlay`), the dimming of the screen below comes from the background gadget (profiled, low).

## Invariants

1. Every popup layout has `lblMessageText` and the answer gadgets named in the table of section 1; every answer event of section 2 is the suffix of at least one handler name elsewhere in the family (script: `OnYes` 40, `OnNo` 26, `OnOk` 6, `OnCancel` 7, `OnAccept` 9, `OnSuccess` 12, `OnFailed` 12).
2. The back command (12) never closes `YesNo`, `Ok`, `Cancel` or `PleaseWait` popups (read; one shared function).
3. The timed popup's limit is 10 seconds (read: the constant is the float 10.0).
4. A polled progress popup ends exactly once: success (1), finished loading (3) or failure (other non-zero) (read; `StepPolled` has no other exit).
5. `CMSLoading` and `CPopupMenuPleaseWaitLarge` are the only two classes that derive from another menu class (`menu-classes.md` section 2).

## Open questions

1. The name resolution function that maps `SPopupMenuYesNo` and the other open names to a layout, and the answer delivery: T01-197.
2. The base initialisation slot called by `PostCreate`, the `Large` classes without methods, `StepTimed` details, and the caller of the timed popup: T01-203.
3. The level information record (`CMenuLevelInfo`): its picture path, the two names, the loading model and description: T01-200.

## Test vectors

1. **YesNo** (read): `CMainMenu::btnQuit_OnClick` opens `SPopupMenuYesNo` with message `btnQuit`, text "Are you serious?", focus `btnNo`; the player presses yes: `btnYes_OnClick` raises `OnYes`; the main menu's `btnQuit_OnYes` runs. Pressing the back command (12) does nothing.
2. **Timed** (read): opened at t = 0 with the limit 10.0; at t = 10.0 + a frame the popup raises `OnNo`; at t = 7.5 the label shows the message and 2.5 seconds left.
3. **Progress** (read): `StartPolledProgress` with a callback that returns 0, 0, 1: `OnSuccess` is raised in the third frame; with 0, 3: `FinishedLoading`, and the bar is 1.
4. **Timed progress minimum** (read, low): `StartTimedProgress(0.05, 0.01, f)` stores minimum 0.1 and maximum 0.1.
