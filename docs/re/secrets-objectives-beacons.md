# Secrets, mission objectives and navigation beacons: rules (T01-170)

- **Roadmap task:** T01-170 (follow-up of `mechanism-classes-triggers.md` open question 2). Classes, members and counts: [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md) sections 5 to 7. The restore of these three kinds by boot type, the chapter save and the unable-to-resurrect list: [`chapter-restore.md`](chapter-restore.md) and [`chapter-restore-players.md`](chapter-restore-players.md).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the two enumerations (the binary's value tables, and every transition in the code uses the same numbers) and for the data tables (all 59 worlds, script `tools/re/secrets_objectives_verify.py`); **medium** for every rule below (one reading of the code each; the names of scheduled functions and the lost arguments recovered from the instruction listing); **low** for the points marked "open". **Nothing was run on the original game.**

Prerequisites: [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md), [`chapter-restore.md`](chapter-restore.md), `trigger-area-rules.md` (a detector's `Activated` event is how scripts reach these entities), `hud-classes-params.md` and `hud-classes-special.md` (the scanner and Netricsa elements), `player-rules.md` (the beacon and objective marker timings), `level-logic.md` (the script commands).

## Summary

1. **Enumerations (high).** `SecretEntityState`: **0 `SES_NONE`**, **1 `SES_FULFILLED`**. `MissionObjectiveState`: **0 `MOS_NONE`**, **1 `MOS_ACTIVATED`**, **2 `MOS_ACCOMPLISHED`**, **3 `MOS_FAILED`**. The navigation beacon has no enumeration: an **active** flag (0 or 1).
2. **A state change is deferred by 0.00001 s.** `SetState(n)` stores the state at once and then schedules the entity's state function (`StateFulfilled`, `StateActivate`, `StateAccomplished`, `StateFailed`) **0.00001 s later** (the same "next think" hop the spawner uses). The state is therefore visible to scripts immediately, the announcement a moment later.
3. **Secrets count once.** `MarkFulfilled` does nothing when the state already is 1; otherwise the host adds 1 to `gs_ctSecrets` and sets state 1. The **total** is the number of secret entities: every secret adds 1 to `gs_ctSecretsTotal` when the game totals are calculated. Nothing in the code ever sets a secret back to 0 except the restore from the chapter save (section 2.4).
4. **Objectives announce only what is configured, and the retail data configures nothing.** The activation announces only when the **spoken text** `ep_strObjAddedToSay` is not empty (it is **empty in all 131 objectives**) and accomplishing or failing announces only when the notify flag is set (it is **0 in all 131**) and the spoken text is not empty. So **in the retail game no objective ever shows an icon, says a line or plays a jingle**; an objective is visible only in the Netricsa mission list, as a world marker (25 of 131) and on the scanner.
5. **Objective slots.** The HUD slot (`en_iRenderingIndex`) of an objective is **1 + the largest index of all objectives** (0 for the first), assigned when it is activated, or when it is accomplished or failed without having been activated.
6. **Beacons are a flag**: `AddBeacon` sets it, `SetCurrentBeacon` clears every beacon's flag and sets this one's. Active beacons draw a marker; the **Use** key makes markers (beacons and active objectives) visible for **4 s** followed by a **3 s** fade.
7. **The chapter save keeps one record per entity**: secrets (id, state), beacons (id, active), objectives (id, state, slot); applied after a boot only when the record's level name and chapter id match (`chapter-restore-players.md` section 3.3).

## Evidence

- **Code.** `CSecretEntity`: `SetState`, `MarkFulfilled`, `StateFulfilled`, `SendSecretFulfilledNotification`, `OnNetricsaIconAppeared`, `SpeechFinished`, `ContributeToGameTotalStats`, `OnShutDown`, `OnDelete`, `PreSendUpdate`, `PostReceiveUpdate`, `OnCreate` (head); `CMissionObjectiveEntity`: `Activate`, `MarkDone`, `MarkFailed`, `SetState`, `StateActivate`, `StateAccomplished`, `StateFailed`, `SendAccomplishedNotification`, `SendFailedNotification`, the three `OnNetricsaIconAppearedAfter...`, `SpeechFinished`, `OnShutDown`, `PreSendUpdate`, `PostReceiveUpdate`, `OnCreate`; `CNavigationalBeaconEntity`: `AddBeacon`, `SetCurrentBeacon`, `OnCreate`; `CWorldInfoEntity`: `CollectSecretEntities`, `CollectMissionObjectives`, `CollectNavigationalBeacons`, `DeactivateAllNavigationalBeacons`, `GetRenderingIndexForObjective`; `CGameInfoEntity`: `GenerateChapterSaveGame`, `RestoreSavedSecrets`, `RestoreSavedObjectivesAndNavBeacons`, `DisableChapterResurrection`, `OnBoot`; the HUD side: `CPuppetEntity::RenderMissionObjectives`, `RenderNavigationalBeacons`, `CPlayerPuppetEntity::GetBeaconsAndObjectivesFadeRatio`, `CPlayerBrainEntity::OnShowWayPoint`, `CNetricsaHudElement::RenderObjTextBox`, `CScannerHudElement::Render`. The two enumerations are static tables of the DLL (value, name); the member names were matched to byte positions in the binary's member tables (`en_`, `ep_`, `sed_`, `nbd_`, `mod_`, `scs_`, `gs_`). The game info's three "announce" slots were identified by their positions in the class's virtual table: the speech notification, the show-icon and the hide-icon functions.
- **Data.** `python -I tools/re/secrets_objectives_verify.py` (about 10 minutes): all placed secrets, objectives and beacons and their records, member by member, and the per-world counts.
- **Not checked.** Nothing was run. What `SpreadSpeechNotificaton`, `ShowNetricsaIconInternal` and the player's `OnSecretFulfilled` do to the player, the exact geometry of the HUD markers and of the scanner, the sort order of the Netricsa page on real data.

## Layout / rules

### 1. Shared mechanics

- **Collections.** At every simulation start and after every world change the world info collects the three kinds into three arrays (`en_cenSecrets`, `en_cenMissionObjectives`, `en_cenNavBeacons`), **one pointer per entity in the world's iteration order**; the arrays are emptied in the pre-shutdown step of a reset and **not** rebuilt by it (the entities persist).
- **No boot and no shutdown reset.** None of the three classes has an `OnBoot` (the slot is the engine's no-op) and the shutdown of the secret and of the objective only **stops their sound**; the beacon has neither. **No reset or restore changes a state except the chapter save restore** (section 5). A state set by a script stays until a restore overwrites it.
- **Net behaviour.** All three are `NEB_STATIC`. The secret and the objective keep a **replicated copy** of the state (`en_sesNetState`, `en_mosNetMissionObjectiveState`): the server copies the state into it just before each update is sent; on a client the **first** update only stores the state (a flag `en_bNeverReceivedUpdate` suppresses the announcement for what happened before the client joined), every **later** change calls `SetState` with the received value, so a client **runs the same state function** (the announcement is played locally). The data holds stray values in the replicated copy of 36 objectives (`mechanism-classes-triggers.md` Summary 5); they are overwritten by the first server update.

### 2. Secrets (`CSecretEntity`)

#### 2.1 The flow of `MarkFulfilled(finder)` (script command)

1. If `en_sesState` is already 1: **nothing happens** (a second call, from any detector, is ignored).
2. `en_henFinder` := the argument (the player who found it; a script may pass none).
3. **Host only:** the session statistics' `gs_ctSecrets` += 1.
4. `SetState(1)`: `en_sesState` := 1 and `StateFulfilled` is scheduled 0.00001 s later.

#### 2.2 `StateFulfilled` and the announcement chain

1. The game info entity's **`DisableChapterResurrection(this secret)`** is called: the secret's entity id is added to the chapter save's unable-to-resurrect list (so a respawn does not re-boot it; `chapter-restore.md` section 3.3). Together with point 3 of the summary this is why a **respawn keeps found secrets found**.
2. If there is no finder: the **closest player** to the secret's position (any distance up to 3 x 10^38) becomes the finder; **if there is no player at all the function ends here** (no announcement).
3. If `en_bPlayHappyMusic` is set (**1 in all 110 secrets**) the jingle (`en_pbsHappyMusic`, `Secret_Found.wav`) is played on the secret's sound source and `SendSecretFulfilledNotification` is scheduled **`en_fHappyMusicPeriod`** seconds later (**0** in all 110); otherwise it is called at once.
4. `SendSecretFulfilledNotification`: if the finder exists: the finder's Netricsa icon is shown (`ShowNetricsaIconInternal`), the finder's `OnSecretFulfilled(this secret's id)` is called, and **`OnNetricsaIconAppeared`** is scheduled **`en_fNetricsaIconAppearingPeriod`** seconds later (**0** in all 110).
5. `OnNetricsaIconAppeared`: the finder's Netricsa **says** the message (`en_strMessage`, shown for `en_fMessageStay` seconds, with the speech line `en_pbsSpeechSound` when set) and `SpeechFinished` is scheduled **`en_fMessageStay + 0.5`** seconds later.
6. `SpeechFinished`: hides the finder's Netricsa icon (`HideNetricsaIconInternal`).

So a secret announces **only to the finder** (not to the other players), and the visible message time is `en_fMessageStay` (**3 s** in 108 secrets, 4 and 6 in the others) plus half a second of icon.

#### 2.3 Counting

`ContributeToGameTotalStats` (called for every `SBaseEntity` by `CalculateGameTotalStats`, `chapter-restore.md` step 2) adds **1** to `gs_ctSecretsTotal`. So the level's total is **the number of placed secret entities** (per world in the data, section 6); the total is rebuilt (the statistics block is reset first) on every reset whose boot type is 0, 5, 7, 9 or 10 and for boot type 1 at the first chapter. The found counter `gs_ctSecrets` is rebuilt by `BootPlayerOnChapter` (column B, `chapter-restore-players.md` section 2.2): taken from the chapter save for boot types 1 and 3, **zeroed** for 0, 5, 7, 9 and 10, and left alone for 2, 4, 6, 8 and 11.

#### 2.4 Restore (the only reset of a secret's state)

`RestoreSavedSecrets` runs from the game info's `OnBoot` for boot types **0, 1, 3, 5, 7, 9, 10** and only when the chapter save's current-chapter record has **the running level's name and the booting chapter's entity id**. For every entry (secret entity id, `sed_sesState`) of the record it looks up the entity by id and, if it is a secret, **writes the stored state straight into `en_sesState`** (no `SetState`, so no announcement, no counter change, no net copy until the next update). A secret that is **not in the record** (created after the record was made, or a record from a chapter that did not list it) keeps its current state.

**Consequence (medium-low, derived from the code):** because the restore is the only reset, after a **chapter restart** (boot type 3) the secrets found *in this chapter* become unfound again (the record was made at the chapter's start) and the found counter returns to the record's value (column B = 1); after a **level restart** (boot type 5) the restore only happens when the player is still **in the first chapter** (the record's chapter id must be the first chapter's), otherwise **every secret keeps its state** while the found counter is zeroed: in that case the found secrets cannot be found again (`MarkFulfilled` returns at step 1) and the counter stays below the total. Not run on the game (T01-174).

### 3. Mission objectives (`CMissionObjectiveEntity`)

#### 3.1 State transitions (script commands)

| Command | Slot assignment | `SetState` | State function (0.00001 s later) |
|---|---|---|---|
| `Activate` | `en_iRenderingIndex` := `GetRenderingIndexForObjective()` (always) | 1 | `StateActivate` |
| `MarkDone` | the same, **only when the state is 0** | 2 | `StateAccomplished` |
| `MarkFailed` | the same, **only when the state is 0** | 3 | `StateFailed` |

There is **no check of the previous state**: `Activate` after `MarkDone` sets 1 again and moves the objective to a new slot; `MarkDone` after `MarkFailed` sets 2 (the slot is kept). `SetState(0)` stores 0 and schedules nothing. At creation the state is set to 0 and the slot to -1.

**`GetRenderingIndexForObjective()`** (world info): the maximum of `en_iRenderingIndex` over **all** collected objectives (the minimum is -1), plus 1. So the first objective ever activated gets slot **0**, the next 1, and so on; an objective that is activated again takes the **maximum + 1** (which can be its own old slot + 1 if it held the maximum).

#### 3.2 The state functions

- **`StateActivate`**: with no game info entity yet it sets the state to 1 again (which schedules itself again 0.00001 s later: it **polls** until the game info exists). Otherwise, **if the spoken text `en_strObjAddedToSay` is not empty**: the game info's **show-icon** call, then `OnNetricsaIconAppearedAfterActivation` is scheduled **`en_fNetricsaIconAppearingPeriod`** seconds later (**0**). That callback sends the **speech notification** (the spoken text with the added sound `en_pbsObjAdded`) and schedules `SpeechFinished` **`en_fObjAddedStay + 0.5`** seconds later (**2.5 s** in all 131); `SpeechFinished` calls the game info's **hide-icon**. With an empty spoken text nothing is announced.
- **`StateAccomplished`** / **`StateFailed`**: **if the spoken text (`en_strObjAccomplishedToSay` / `en_strObjFailedToSay`) is empty or the notify flag (`en_bNotifyObjAccomplished` / `en_bNotifyObjFailed`) is 0**, the game info's hide-icon is called and the function ends: **a silent state change**. Otherwise: if `en_bPlayHappyMusic` / `en_bPlaySadMusic` is set the jingle (`Mission_Success.ogg` / `Mission_Fail.ogg`, or the override) is played and `SendAccomplishedNotification` / `SendFailedNotification` is scheduled after `en_fHappyMusicPeriod` / `en_fSadMusicPeriod`; else it is called at once. The send function shows the icon and schedules the `...AfterAccomplished` / `...AfterFailed` callback after `en_fNetricsaIconAppearingPeriod`; that sends the speech notification and schedules `SpeechFinished` after `en_fObjAccomplishedStay + 0.5` / `en_fObjFailedStay + 0.5` seconds.

In the retail data the activation's spoken text is **empty in all 131 objectives** and **both notify flags are 0**, so **none of the three state functions announces anything**: each only stores the state (and re-hides the icon).

#### 3.3 Where the state shows

- **Netricsa mission page** (`CNetricsaHudElement::RenderObjTextBox`, title "MISSION OBJECTIVES"): the world info's objective array is **sorted in place by `en_iRenderingIndex`, largest first** (the comparison is "second minus first"), then every objective whose state is **not 0** is listed with its **added text `en_strObjAdded`** (the displayed objective text; wrapped to the box width), at full colour when the state is **1** and at **half transparency** (alpha 0x80) when it is 2 or 3. So accomplished and failed objectives stay listed, dimmed, **newest slot at the top**. The page scrolls.
- **World markers** (`CPuppetEntity::RenderMissionObjectives`): for every objective whose state is **1** and whose `en_bVisibleOnScanner` is set (**25 of 131**): the objective's position is projected to the screen (nothing is drawn when it is behind the camera); with d the distance from the camera in world units, the marker appears only when **d is beyond the hot spot `ggp_fBeaconAndObjFadeOutHotSpot` (20)** and its opacity rises linearly to full at **`ggp_fBeaconAndObjFadeOutFallOff` (40)**; the icon is `ggp_ptxObjCrosshair`, drawn rotated, sized `ggp_pixObjCrosshairSize` (100) times a **1 Hz pulse** (sin of 2 pi times the seconds since the Use key was pressed, mapped to 0..1) with the colour orange (0xFF7F3F), and the distance (whole units) is written below it.
- **Scanner** (`CScannerHudElement::Render`): an objective is drawn when its state is **1 or 3** (not 0, not 2) and `en_bVisibleOnScanner` is set; state 1 in the objective colour `shep_colObjectives` (0xF7FF9B30), state 3 in `shep_colObjFailed` (0xFFFF66FF); inside the scanner radius as a blip, outside it as an arrow on the rim.
- **Visibility timing of the markers.** The markers (objectives and beacons alike) are multiplied by `GetBeaconsAndObjectivesFadeRatio` of the player puppet: with t the seconds since the player's brain stamped `en_tmLastPressedWayPoint` (**the Use command, labelled "Use/Show way points"**, `CPlayerBrainEntity::OnShowWayPoint`; the stamp is -100 s at every boot): **1 while t < `ggp_fBeaconAndObjStayTime` (4)**, then **(7 - t) / 3** until `ggp_fBeaconAndObjStayTime + ggp_fBeaconAndObjFadeOutTime` = 7, **0 after**. A puppet that is not a player has ratio 0. So **the markers are visible only for 7 seconds after the player presses Use**; the scanner does not use this timing.

### 4. Navigation beacons (`CNavigationalBeaconEntity`)

- `en_bActive` is 0 at creation. **`AddBeacon`**: `en_bActive` := 1. **`SetCurrentBeacon`**: first `DeactivateAllNavigationalBeacons` (every collected beacon's flag := 0), then this beacon's flag := 1. There is **no command that clears one beacon** except `SetCurrentBeacon` on another. A beacon's `en_bDirectionOnly` is copied from its record (**171 of 174** set).
- **World markers** (`CPuppetEntity::RenderNavigationalBeacons`): the same rule as the objective markers for every **active** beacon (d beyond 20, full at 40, the fade ratio of the Use key above), with the icon `ggp_ptxNavBeaconCrosshair` of size `ggp_pixNavBeaconCrosshairSize` (100), pulsing, in yellow (0xFFFF00). The direction-only flag is **not** read here.
- **Scanner:** every active beacon is drawn in `shep_colBeacon` (0xFFFFD800). A beacon **inside** the scanner radius is drawn as a blip at its position **unless it is direction-only**, in which case it is drawn as the **rim arrow** like the beacons outside the radius (so a direction-only beacon shows a **direction, never a position**).

### 5. The saved state (`CSecretEntityData`, `CNavigationalBeaconData`, `CMissionObjectiveData`)

Per chapter record (`scs_asedSecretEntities`, `scs_anbdNavigationalBeacons`, `scs_amodMissionObjectives`; `chapter-restore-players.md` section 3.1):

| Entry | Members (in this order) | Source at save time | Applied at restore |
|---|---|---|---|
| secret | `sed_ulSecretID` (the secret's entity id), `sed_sesState` | `en_sesState` | written into `en_sesState` |
| beacon | `nbd_ulNavigationalBeaconID`, `nbd_bActive` | `en_bActive` | written into `en_bActive` |
| objective | `mod_ulMissionObjectiveID`, `mod_mosMissionObjectiveState`, `mod_iRenderingIndex` | `en_mosMissionObjectiveState`, `en_iRenderingIndex` | both written back (state and slot) |

(`save-game.md`'s "two further values" for the objective are the state and the slot; the order **state, slot** is the data's member order and the code's.) The arrays are **rebuilt from the collected entities on every chapter start**: one entry per entity, in array order, so after the Netricsa page has sorted the objective array (3.3) the objectives are saved **sorted** (order is irrelevant for the restore, which is by id). An entity that is in the record is restored when the boot type allows: objectives and beacons for boot types **1, 2, 3, 4**, secrets for **0, 1, 3, 5, 7, 9, 10** (`chapter-restore.md` section 1); the guard is the record's level name and chapter id (`chapter-restore-players.md` 3.3). The restore also **does not run the state functions**: a restored objective is neither announced nor re-slotted. **Not restored:** the replicated copies (they follow at the next update), the secret's finder, any running announcement.

**Per boot type, in short:**

| Boot type | Secrets | Objectives and beacons |
|---|---|---|
| 0 new game, custom | restored (nothing to restore for a new world) | not restored |
| 1 continue | restored | restored |
| 2 respawn | not restored; found secrets are kept (and the secret entity is not re-booted) | restored |
| 3 restart chapter | restored | restored |
| 4 | not restored | restored (never used) |
| 5 restart level | restored when the player is in the first chapter, else nothing changes | not restored |
| 6, 8, 11 | not restored | not restored |
| 7, 9, 10 | restored if the record matches | not restored |

## Invariants

1. Every placed secret, objective and beacon has exactly one record and one entity; at the start every secret is in state 0, every objective in state 0 with slot -1, every beacon inactive (data census, `tools/re/secrets_objectives_verify.py`).
2. `en_sesState` is 0 or 1, `en_mosMissionObjectiveState` is 0 to 3 in all code paths; no function writes another value.
3. `ep_fObjAddedStay`, `ep_fObjAccomplishedStay` and `ep_fObjFailedStay` are **2** in all 131 objectives; both notify flags are 0 and `ep_strObjAddedToSay` is empty in all 131 (data).
4. A secret's state function runs at most once per fulfilment (the state is checked before it is set); an objective's state function runs once per command.

## Open questions

1. **The announcement path inside the player** (`SpreadSpeechNotificaton`, `SpreadShowNetricsaIcon`, `SpreadHideNetricsaIcon`, the player's `ShowNetricsaIconInternal`, `SayNetricsaInternal`, `OnSecretFulfilled`): which players hear it, the Netricsa's text queue and the speech sound handling. **T01-345.**
2. **The geometry and numbers of the HUD markers and the scanner**: the pulse and fade constants in pixels, the scanner range and stretch, the rim arrow placement, the distance text format, and the comparison used by the Netricsa page sort when two objectives share a slot. **T01-346** (with `hud-classes-special.md`).
3. **Run-time confirmation of the restore consequences** (found secrets after "Restart level" from a later chapter; the silent accomplish and fail of the retail data; the 7 s window of the Use key): **T01-174.**
4. **The chapter link of these entities** (1 secret, 11 objectives and 15 beacons carry one) is not read by their own code, only by the boot rule (`chapter-restore.md` 3.3), which does not matter for them because they have no `OnBoot`. No question, listed for completeness.

## Test vectors

All binary readings unless a data line is named; nothing was run.

1. **A secret.** A detector over a secret raises `Activated`; the level script calls `MarkFulfilled(player)`: state 1, `gs_ctSecrets` + 1 (host), 0.00001 s later `StateFulfilled`: the secret is registered as unable to resurrect, the jingle plays, the finder's icon appears, the message shows for 3 s + 0.5 s. A second `MarkFulfilled` does nothing.
2. **Slots.** Objectives A, B, C placed with slot -1. `A.Activate` -> 0; `B.Activate` -> 1; `A.MarkDone` -> state 2, slot stays 0 (state was 1); `C.MarkDone` (state 0) -> slot 2, state 2; `A.Activate` -> slot 3 (maximum 2, plus 1), state 1.
3. **Silent objectives (every retail objective).** `Activate`: state 1, slot assigned, `StateActivate` finds the spoken text empty and does nothing; `MarkDone`: state 2, `StateAccomplished` finds the notify flag 0, hides the (not shown) icon and ends; the Netricsa page lists the objective, full colour in state 1, dimmed in 2 and 3.
4. **Beacons.** `P.AddBeacon`, `Q.AddBeacon`: both active; `R.SetCurrentBeacon`: P and Q inactive, R active.
5. **Marker window.** The player presses Use at t = 0: markers at full opacity until t = 4 s, 2/3 at 5 s, 1/3 at 6 s, gone at 7 s.
6. **Restore.** A chapter restart (3) after finding secret S in the chapter: the record (made at the chapter start) has S in state 0 -> S is set back to 0 and can be found again; `gs_ctSecrets` returns to the record's count.
7. **Census.** See "Data" below.

## Data (census of the 59 worlds)

Script `python -I tools/re/secrets_objectives_verify.py` (about 10 minutes), all 59 worlds:

- **Secrets: 110 in 32 worlds** (`CSecretEntity` / `CSecretProperties`; per world 1: 3 worlds, 2: 9, 3: 9, 4: 5, 5: 2, 6: 2, 9: 1, 11: 1; so the level's secret total is 1 to 11, **110 in all**). All start in state 0 (net copy 0), no finder, `en_bNeverReceivedUpdate` 1; **message text set in all 110**; `ep_fMessageStay` 3 (108), 4 (1), 6 (1); `ep_bPlayHappyMusic` 1 and both periods 0 in all 110; **one secret carries a chapter link**.
- **Mission objectives: 131 in 40 worlds** (per world 1: 9, 2: 7, 3: 12, 4: 2, 5: 4, 6: 2, 7: 1, 8: 2, 9: 1). State 0 and `en_iRenderingIndex` -1 in all 131; `en_bVisibleOnScanner` 1 in **25**; the displayed text `ep_strObjAdded` set in all 131; **the spoken text `ep_strObjAddedToSay` empty in all 131**; the accomplished and failed spoken texts set in 124 and empty in 7 each; `...Stay` 2 s in all; `ep_bPlayHappyMusic` and `ep_bPlaySadMusic` 1, both periods 0, `ep_fNetricsaIconAppearingPeriod` 0; **both notify flags 0 in all 131**; the replicated state copy holds stray values in 36 (the rest 0); **11 carry a chapter link**.
- **Navigation beacons: 174 in 29 worlds** (per world 1: 2, 3: 3, 4: 4, 5: 6, 6: 5, 7: 2, 8: 3, 10: 1, 11: 1, 14: 2); `ep_bDirectionOnly` 1 in **171**, 0 in 3; `en_bActive` 0 in all 174; **15 carry a chapter link**.
