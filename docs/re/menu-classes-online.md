# Menu classes: Xbox Live, friends and online player screens (T01-112, page 5 of 6)

- **Roadmap task:** T01-112 (overview: [`menu-classes.md`](menu-classes.md); this page covers the 15 classes of the "online" group)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, layouts, gadget names and handler-to-gadget matches (script); **medium** for the few rules marked "read"; **low** for "(profiled)" purposes (most functions here are short answers that call online-service functions of the engine whose names were not mapped) and for the Open questions. Nothing was run.

Prerequisites: [`menu-classes.md`](menu-classes.md) (popup protocol), `gui.md` (platform masks: the Xbox gadgets are hidden on PC), `replication.md`.

## Summary

The online group is the **Xbox Live and friends front end**: sign-in and account screens, the friends and player lists, messages with voice attachments, feedback and complaints about other players. 15 classes, **234 exported functions** (105 standard + 129 named), 13 with named methods. In the PC build these screens are **not reachable**: the main menu's `btnXboxLive` and `btnSystemLink` and the in-game `lnkXboxLive` and `lnkPlayerList` carry platform mask 2 (Xbox only: script on the layouts, `gui.md`); the PC player list is `CMSPlayerList` (`menu-classes-session.md`). The PC engine still ships the code and layouts. A port that targets PC can leave the whole group out (T01-202 only if the Xbox build matters).

## Evidence

Export names and layouts (`tools/re/menu_classes_verify.py`); profiles of all 129 named methods (strings and callees); no function of this group was decompiled in full. Callees are engine online functions (`onl...`: login, connect, account list, friends) which were not mapped, so purposes are **profiled** at best. **Not checked:** every service call, the message and voice formats, the account and friend list contents.

## Layout / rules

### 1. Flow and the two common patterns

```
CMainMenu --btnXboxLive--> CMSXboxLive --btnSignIn--> CMSOnlineLogin (account list, pass code) 
CMSXboxLive --lnkPlayLive--> CMSProfileSelection (caller lnkPlayLive) --> CMSXboxPlayLive --> CMSOptiMatch / SMSQuickMatch / session settings
CMSXboxLive --lnkPlayers--> CMSOnlinePlayerList ; --lnkFriends--> CMSOnlineFriendList ; --lnkXboxLiveOptions--> CMSXboxLiveOptions
friend list item --> CMSFriendsOptions ; player list item --> CMSPlayerOptions --lnkPlayerFeedback--> CMSPlayerFeedback --lnkPlayerComplaint--> CMSPlayerComplaint
```

(links from the data: `menus-tables.md`; item-to-options transitions profiled.)

**The "required actions" pattern.** `CMSOnlineLogin` and `CMSOnlineAcceptGameInvite` each export the same **six answer pairs** (12 names) of the online sign-in: `AccountLogin_OnSuccess/_OnFailed`, `AccountManagement_OnYes/_OnNo`, `AutoUpdate_OnYes/_OnNo`, `GetPassword_OnAccept/_OnCancel`, `MessagePending_OnYes/_OnNo`, `NetworkProblems_OnYes/_OnNo`, and `HandleRequiredActions` (a 5-byte stub in both). They answer the popups that the engine's login asks in turn: the account needs management, an update is required, a pass code is needed, a message is pending, the console cannot reach the service. Most of these are 5 to 17 bytes (a call or nothing); the three `_OnNo` answers of `AccountManagement`, `AutoUpdate` and `NetworkProblems` are 17-byte functions of one shape (profiled). `CreateNewAccount_OnYes` (login only) shows `Your Xbox console cannot connect to Xbox Live. Do you want to start the troubleshooter?` (message `NetworkProblems`).

### 2. Classes

| Class | Layout (gadgets) | Functions (named) | What they do |
|---|---|---|---|
| `CMSXboxLive` | `cmsxboxlive` (11: `btnSignIn`, `btnSignOut`, `btnXboxDashboard`, links `lnkPlayLive`, `lnkPlayers`, `lnkFriends`, `lnkXboxLiveOptions`) | 18 (11) | The Live hub. `UpdateState` (399 bytes, every frame through `Step`) enables **sign-in when signed out and sign-out, dashboard and the four links when signed in**; `OnSetCurrent` (177) opens `CMSOnlineLogin` when not yet signed in; `btnSignIn_OnClick` opens it too (caller `btnXboxLive`); `btnSignOut_OnClick` signs out; `btnXboxDashboard_OnClick` and its `XboxDashboard_OnYes/_OnNo` are empty (1-byte) stubs here; `NetworkProblems_OnYes` calls one function. `PostCreate` focuses `btnSignIn` or `lnkPlayLive`. |
| `CMSXboxLiveOptions` | `cmsxboxliveoptions` (6: `opslAppearOffline`, `opslVoiceThroughSpeakers`, slider `sldVoiceVolume`) | 13 (6) | Online profile options: `PostCreate` (329 bytes) loads the three values from the online profile; `UpdateState` keeps the slider enabled only when voice through speakers is on (profiled); the `_OnChange` handlers write them back (the option texts `ETRSProfile.Option.Yes` are compared). |
| `CMSXboxLivePassEntry` | `cmsxboxlivepassentry` (2: a label and the gadget `gXboxLivePassEntry`, popup underlay) | 13 (6) | The **pass code entry**: the Live account pass code is a **sequence of controller buttons** (D-pad down, left, right, up, X, Y, left and right trigger; the strings are `ButtonDown`, `ButtonLeft`, `ButtonUp`, `ButtonX`, `ButtonY`, `GamepadX`, `LeftTrigger`, `RightTrigger`). `OnSetCurrent` (1,319 bytes) **registers eight system commands** `syscmdXLPassDown`, `Left`, `Right`, `Up`, `X`, `Y`, `LTrig`, `RTrig` bound to those buttons (the console logs `Xbox Live password entry, button D_Down.` and so on) and `OnUnsetCurrent` unregisters them; `OnAnyCommandPressed` feeds each command to the gadget; `OnCommandPressed` on cancel raises `OnCancel`, on completion stores the code in the variable `AccountPassword` (profiled). Declared base: the only screen that overrides slot 25. |
| `CMSXboxPlayLive` | `cmsxboxplaylive` (6: links `lnkQuickMatch`, `lnkOptiMatch`, `lnkCreateAMatch`) | 8 (1) | Match entry; `PostCreate` focuses `lnkQuickMatch`. |
| `CMSOnlineLogin` | `cmsonlinelogin` (3: `lstOnlineAccountList` of the kind `OnlineAccountList`, a label `lblNoAccount`; large popup underlay) | 25 (18) | **Account choice.** `Step` (190 bytes) shows `lblNoAccount` when the account list is empty; `OnCommandPressed` (632 bytes) on accept opens a PleaseWait popup "Signing in" (message `AccountLogin`) and, when the account has a pass code, first opens `CMSXboxLivePassEntry` with the text "Please enter the pass code for %1" (message `GetPassword`); see the pattern above. |
| `CMSOnlineAcceptGameInvite` | `cmsonlineacceptgameinvite` (4: `btnYes`, `btnNo`) | 26 (19) | A **game invitation** arrived. `PostCreate` shows "%1 has accepted a game invitation. Do you want to join the session?"; `btnYes_OnClick` (496 bytes) signs the inviting account in (no such account: OK popup "The account that accepted the game invitation doesn't exist.", message `NoAccount`; pass code: `CMSXboxLivePassEntry`) and then `JoinGame`; `JoinGame` (218 bytes) opens a PleaseWait popup "Connecting to host" (message `ConnectingToHost`) and, with the **cable disconnected**, the message `Message.Network.CableDisconnected`; `ConnectingToHost_OnSuccess` (148) opens the loading screen, `_OnFailed` shows `This game session is no longer available.` |
| `CMSOnlineFriendList` | `cmsonlinefriendlist` (4: `lstFriendsList`) | 7 (0) | **No method**: the list gadget and the layout's links do all of it. Item selection opens `CMSFriendsOptions` (not seen in this class; profiled in the other class's strings: T01-202). |
| `CMSOnlinePlayerList` | `cmsonlineplayerlist` (4: `lstOnlinePlayerList`) | 7 (0) | **No method**; opens `CMSPlayerOptions` for the chosen player (as above). |
| `CMSFriendsOptions` | `cmsfriendsoptions` (16: 11 buttons) | 32 (25) | What can be done to one friend. `UpdateState` (662 bytes, every frame) enables exactly the buttons that apply to the friend's state: `btnSendGameInvitation`, `btnAcceptGameInvitation`, `btnDeclineGameInvitation`, `btnCancelGameInvitation`, `btnAcceptFriendRequest`, `btnDeclineFriendRequest`, `btnBlockFriendRequests`, `btnPlayVoiceAttachment`, `btnMessageFeedback`, `btnJoinFriend`, `btnRemoveFriend` (11 of 11 in the layout). Yes/no confirmations ("Are you sure you want to block friend requests from %1?", "...cancel the game invitation to %1?", "...remove %1 from your friends list?"; messages equal the buttons) guard the three destructive ones. `JoinFriend` (567 bytes) connects to the friend's session (`INSERT GAME DISC%1` text for a missing disc, the cable check, a Cancel popup and a PleaseWait popup `ConnectingToHost`); `btnSendGameInvitation_OnClick` opens `CMSOnlineSendMessage` with `OnlineMessageType` set; `btnPlayVoiceAttachment_OnClick` opens `CMSOnlineVoicePlayback` with `VoicePlaybackPurpose`; `btnMessageFeedback_OnClick` opens `CMSOnlineMessageFeedback`; `PostCreate` shows the friend's name (`SelectedFriendName` into `lblFriendName`); `SetMessageAsCurrent` marks the pending message. |
| `CMSPlayerOptions` | `cmsplayeroptions` (10: `btnSendFriendRequest`, `btnCancelFriendRequest`, `btnMutePlayer`, `btnUnmutePlayer`, link `lnkPlayerFeedback`) | 14 (7) | The same for a player in the session: `UpdateState` enables send or cancel friend request and mute or unmute; `btnSendFriendRequest_OnClick` opens `CMSOnlineSendMessage` (a friend request message); `PostCreate` shows `SelectedPlayer`. |
| `CMSPlayerFeedback` | `cmsplayerfeedback` (7: `btnFeedbackGoodAttitude`, `btnFeedbackGoodSession`, link `lnkPlayerComplaint`) | 10 (3) | Positive feedback: each button sends it and shows the OK popup "Player feedback sent." (message `Feedback sent.`). |
| `CMSPlayerComplaint` | `cmsplayercomplaint` (9: `btnFeedbackBadName`, `Cheating`, `Screaming`, `Harassment`, `Cursing`) | 13 (6) | Five complaint kinds; each sends it and shows "Player complaint sent." (message `Complaint sent.`); `PostCreate` shows `SelectedPlayer` in `lblPlayerName`. |
| `CMSOnlineMessageFeedback` | `cmsonlinemessagefeedback` (6: `btnFeedbackSpamMessage`, `btnFeedbackOffensiveMessage`) | 10 (3) | Reports a received message (spam or offensive): "Message feedback sent." |
| `CMSOnlineSendMessage` | `cmsonlinesendmessage` (9: `btnCreateVoiceAttachment`, `btnPlayVoiceAttachment`, `btnDeleteVoiceAttachment`, `btnSendMessage`) | 15 (8) | Compose a message to a friend or player (a friend request, a game invitation): `PostCreate` (400 bytes) reads `OnlineMessageType`, `SelectedFriendName` or `SelectedPlayer` into `lblRecipientName`; `UpdateState` enables the voice attachment buttons by whether an attachment exists; create and play open `CMSOnlineVoicePlayback` (purpose in `VoicePlaybackPurpose`); delete drops it; `btnSendMessage_OnClick` (246) sends, and on a **full friend list** shows "Unable to add the specified friend because your friends list is full." (message `FriendListFull`). |
| `CMSOnlineVoicePlayback` | `cmsonlinevoiceplayback` (7: `btnPlay`, `btnRecord`, `btnStop`, `pbProgressBar`; popup underlay) | 23 (16) | Record and play a **voice attachment**: an explicit state (`IsIdle`, `IsInPlayback`, `IsInRecordingMode`); `StartRecording` shows "Recording...", `StartPlayback` "Playing...", `Stop` "Stopped..."; `Step` (213 bytes) advances the progress bar (`StepTimed`: the declared members are a maximum duration `ms_fMaxDuration`, a voice duration `ms_fVoiceDuration`, a start time `ms_tmStartTime`, a finished flag `ms_bFinished`, the current mode `ms_vomCurrentMode` and the purpose `ms_vomPurpose`), logs `Progress bar is missing from the menu screen.` when the bar is absent, and `UpdateState` enables the three buttons by state; command handling stops on the back command (`OnCommandPressed` calls `btnStop_OnClick`). |

## Invariants

1. Of the 81 handler methods of this group, 46 name a gadget of their own layout; the other 35 are answers named after a message (`ConnectingToHost`, `AccountLogin`, ...) or after the button that opened a confirmation (script).
2. Every confirmation popup of `CMSFriendsOptions` uses as its message the name of the button that opened it (`btnBlockFriendRequests`, `btnCancelGameInvitation`, `btnRemoveFriend`), so the answers are `<button>_OnYes` and `_OnNo` (script: names in the export list).
3. The two login screens export the same 12 answer names plus `HandleRequiredActions` and `PostCreate` (script: 14 names in common).
4. The group is reachable only on the Xbox platform mask (script on the layouts: the four entry gadgets `btnXboxLive` and `btnSystemLink` of the main menu and `lnkXboxLive` and `lnkPlayerList` of the in-game menu carry mask 2).

## Open questions

1. What each online service call does and the exact conditions of every `UpdateState` (friend and player states): T01-202.
2. The message, friend and voice formats and the pass code encoding: T01-202.
3. Whether any of this is needed for a PC port (the PC build hides every entry): decided with the owner; T01-202 is parked until then.

## Test vectors

1. **Pass code** (profiled): the eight system commands are bound while `CMSXboxLivePassEntry` is the top screen and unbound when another screen covers it; a code is the sequence of those eight buttons, stored in the variable `AccountPassword` on completion.
2. **Voice attachment** (profiled): `CMSOnlineSendMessage` with no attachment enables only `btnCreateVoiceAttachment`; after `CMSOnlineVoicePlayback` has recorded, `btnPlayVoiceAttachment` and `btnDeleteVoiceAttachment` become enabled.
