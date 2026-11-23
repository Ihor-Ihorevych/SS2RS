# Console variables and commands, start-up scripts, profiles, key binds (T01-033)

- **Roadmap task:** T01-033
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the start-up files, the script syntax and the variable names (read from the shipped scripts and from the modules' name tables); **medium** for which names are user-settable console variables and for the services and commands (names and types are certain; behavior is inferred from names, the help file and the scripts); **low** for the file formats of the profile and the key-binding table (neither is in the install); **open** for the default key assignments and the meaning of most numbers.

Prerequisites: `misc-types.md` (`.cfg` files), `mods.md` (`DefaultMod.txt`), `player-rules.md`, `menus-tables.md`, `gui.md` (menu screens), `localization.md` (string keys). The generated name list is **`console-variables.md`**.

## Summary

The engine has a **console**: a registry of named global variables and callable functions, a small **script language** to set them, and **start-up scripts** (`*.cfg`) that run in a fixed set of module sections. The game keeps **per-player profile settings** (sensitivity, invert look, controller presets, key assignments) in a profile folder created at run time. The install ships only four tiny scripts, one safe-mode settings file and no profile data, so **the default key bindings are not in the data**: this spec gives what is known (the vocabulary, the option lists, the files and the names) and lists what a reimplementation must decide itself (T03 input, T09 menus).

## Evidence

- The five `Config/` scripts and `DefaultMod.txt`, read in full (sizes below).
- **Names** from the modules: the decorated C++ names that `Core.dll`, `Engine.dll`, `ProcRender.dll`, `Shaders.dll`, `Sam2Game.dll`, the graphics and sound back-ends export (a global variable's decorated name carries its type: float, int, long, unsigned long, bool), and plain name strings inside four of them. **Names and types only**: no code, no values other than those of the shipped scripts (`conventions.md` §5).
- File names the executables refer to (strings of `Sam2.exe`, `DedicatedServer.exe`, `Core.dll`, `Sam2Game.dll`).
- The editor help (`help-digest.md`) for the console line of the editor and the profiler variables.
- The menu layouts' text keys (`localization.md`) for the profile options.

Nothing was run; no values were read from process memory.

## Start-up files

| File | Where | Size | Role |
|---|---|---|---|
| `autoexec.cfg` | `Content/SeriousSam2/Config/` (in `Patch_02_068.gro`) | 192 bytes | start-up script of the PC game: starts the two **local remote-console services** (below) |
| `autoexec_xbox.cfg` | same | 915 | the console build's script: stealth mode, FPS and memory display, a small remote-console buffer, a file-based remote console, lower crumb density (distance bias 0.8, density bias 0.6), **4 player profiles** and a vertical look sensitivity of **0.75** |
| `user.cfg` | same, **loose** in the install | 35 | one assignment: an empty remote-console **administrator password** (`rcts_strAdminPassword`); the place for user-written settings |
| `SafeMode.cfg` | same, **loose** in the install | 2,974 | **112 variable assignments** (`[Engine]` 72, `[Sam2Game]` 29, `[ProcRender]` 6, `[Shaders]` 5): a minimal-quality, windowed, 1024 × 768 profile for recovering from a bad video setting |
| `boot.cfg` | same | _absent_ | named in the Xbox script ("if changing of stealth mode isn't locked by `boot.cfg`") and in both executables: an optional script that runs **before** the others; it does not ship |
| `Sam2.ini`, `DedicatedServer.ini`, `Content/<Mod>/<Mod>.ini` | `Content/SeriousSam2/`, `Content/%1/` | _absent_ | the executables look for an `.ini` for the game (client), for the dedicated server and for the selected mod (`%1` is the mod name); none ships |
| `DefaultMod.txt` | `Content/` (in `Patch_02_068.gro`) | 313 | names the **default mod** (below) |
| `CurrentMod.txt` | `Content/` | _absent_ | the user's mod selection; `Core.dll` contains a format string of the form `CURRENTMOD = <name> ;`, so it presumably writes the file |

The order in which the files run is not recorded in the data. The Xbox script's own comment (stealth mode "not locked by `boot.cfg`") shows that `boot.cfg` is read first and can lock settings, and that `autoexec` is read after it; `user.cfg` is presumably last so that its values win. `SafeMode.cfg` is **not** run in a normal start (it is selected explicitly, for example when the video set-up fails).

### `DefaultMod.txt` (and a correction to `mods.md`)

A text file: `#` comment lines saying that the file names the default mod, that it is read both by the build system (to find dependencies) and by the program when loading the selected mod, and that **to switch to another mod you save a copy as `CurrentMod.txt` and change `CURRENTMOD`**; then the single statement `CURRENTMOD = SeriousSam2.mod ;`. So **the mod selection is a one-line config file with the same `NAME = value ;` grammar as `.mod` and `.nfo`**, read from `Content/CurrentMod.txt` if it exists and otherwise from `Content/DefaultMod.txt`. (An earlier version of `mods.md` said the file does not ship; it ships inside `Patch_02_068.gro`. `mods.md` is corrected in the same change.)

## The script language

The scripts use one small language (shared with the `.mod`/`.nfo` family for plain assignments):

```
[Module]                      // a section: the module whose variables follow (Core, Engine, ProcRender, Shaders, Sam2Game)
name = value;                 // numbers (0.25, 1024, 0.8f), quoted strings ("SMainMenu"), no suffix needed on floats
if(!dbg_bLockStealthMode) { dbg_bStealthMode = 1; } else { … }   // C-like conditions on variables
svcStartService(ident("Service.RConServer_Local"));              // a call; ident("…") makes an identifier from a name
// comment to end of line
```

- **Sections** name the module that **owns** the variable (they decide when it is applied: a module's variables exist once the module is loaded, which is why the PC script has only `[Engine]`). Variable families seen per section: `[Core]` `dbg_`; `[Engine]` `gfx_ d3d_ ogl_ tex_ lmp_ mdl_ ren_ sfx_ dsd_ snd_ prf_ rcon_ rcts_`; `[Shaders]` `sha_`; `[ProcRender]` `efx_ crm_`; `[Sam2Game]` `con_ sam_ hud_ men_ plp_ inp_`.
- **Values**: integers, floats (with an optional `f` suffix: `0.8f`), strings in double quotes; booleans are 0 and 1.
- **Statements** end with `;`; blocks use braces; `if (…) … else …` exists; `!` negates. No other statement kinds occur in the shipped scripts.
- **Hungarian type tags** in the names (`b` boolean, `i` integer, `f` float, `str` string, `sl`/`ul` signed/unsigned long, `pix` pixel count, `ct` count, `tm` seconds) say how to parse the value; the decorated export names confirm them (bool/int → `int`, `f` → `float`, `sl` → `long`, `ul` → `unsigned long`).

## Console variables

`console-variables.md` lists **450 names in 40 families** (name, type, the modules it appears in, and the safe-mode value when `SafeMode.cfg` sets it). Counts per family: `gfx` 63, `sam` 51, `net` 46, `phy` 32, `sha` 25, `ren` 21, `hud` 16, `md` 16, `men` 16, `sfx` 15, `d3d` 11, `crm` 10, `dbg` 10, `prf` 10, `tex` 9, `inp` 8, `efx` 7, `sim` 7 and 22 smaller families. What the families are for:

| Family | What it controls (from the names and the scripts) |
|---|---|
| `gfx_ d3d_ ogl_` | window and full-screen mode, resolution (`gfx_pixResWidth/Height`), refresh rate, VSync (`gfx_iWaitVSyncs`), anti-aliasing, HDR, FPS caps for the active and the inactive window (`gfx_iMaxFPSActive` 10,000 and `gfx_iMaxFPSInactive` 20 in safe mode), gamma, brightness, contrast, per-channel bias, occlusion-query count, pixel-program version limits, which graphics API (`gfx_iAPI`, 2 in safe mode) |
| `tex_ lmp_ mdl_ ren_` | texture and light-map quality, filtering, anisotropy, LOD bias, maximum size and base mip; model weights per vertex (safe mode **2**), hardware skinning, animation cache (**64** cached models); occlusion culling, visibility groups, LOD bias, near clip plane (**0.1**), dynamic shadow quality and size (**128** pixels) |
| `sfx_ dsd_ snd_` | sound back-end, EAX, active sounds (**20**), same sound at once (**5**), streaming packets (**8** of **8**), master volume, attenuation by voice (**0.75**), effects/music/movie volume |
| `sha_ efx_ crm_` | shader options (offset mapping, refractions, cube reflections), particle and bloom bias, lens-flare quality (**3**), "crumbs" (small ground decoration) density, distance and caching |
| `con_ ccn_` | the in-game console and chat console: lines shown (**10**), height ratio (**0.25**), line life (**5 s**) and fade (**2 s**) |
| `hud_ men_` | what the HUD shows (crosshair, weapon, info, coordinates, ping, bandwidth, time, copyright, logo), menu safe zone, attract mode, menu music volume |
| `sam_` | game options: movies skipped, boot sequence, debris on/off and **maximum debris (20)**, ragdoll deaths, subtitles and voices for Sam, Netricsa and NPCs, server name, map list and map directory, start-as-server, connect string, level to start, demo file and loop, cheat level (`sam_iEnableCheats`), voting (duration, percentage), disabled items and vote types |
| `net_ cli_ ser_ rcts_ rcon_` | networking: ports, passwords (connect, observer, VIP, admin), maximum clients and observers, bandwidth limits (`cli_iMaxBPS`, `cli_iMaxOutBPS`, low/high-quality BPS), timeouts, packet-loss and latency emulation, update time limits, compression, ban list (`ser_strBanList`), Steam game server and VAC flags, the remote-console administrator password and welcome note |
| `phy_` | the physics solver: iterations, sub-step size, sleeping thresholds, warm starting, friction and bounce switches, thread count (`physics.md`) |
| `prf_ dbg_ mmp_ mmt_ fil_ thr_ md_ str_` | profiler and FPS display (`prf_bShowProfiler`, `prf_bShowFPS`, `prf_strOutputFilter`, `prf_tmRefreshInterval`), debug and stealth modes (`dbg_bStealthMode` = no log or report files), memory tracking, file system (`fil_bPreferZips`, write-cache size), worker threads (`thr_iMaxWorkerThreads`), data-type system switches (`md_bAllowRawLoading`, `md_bSaveTextFiles`: how resources are read and written), locale (`str_strCurrentLocale`, `str_strLocaleDir`, `str_strTransPrefix`: `localization.md`) |
| `plp_ inp_` | player profiles (`plp_iMaxPlayerProfiles`, `plp_strActiveProfile`, `plp_bClearAchievementProgress`) and input (`inp_fMouseSensitivity`, `inp_fYLookSensitivity`, `inp_bInvertMouseY`, `inp_bMouseSmoothing`, `inp_iControllerDeadzone`, `inp_bGamepadChecks`) |
| `sim_` | the simulation time factor (`sim_fRealTimeFactor`) and similar |

Two details matter for a reimplementation:

- **Named settings exist for the user's graphics and sound choices**; a reimplementation's settings schema (T03-014) can reuse the *names and meaning* of the useful ones, but need not copy the whole list (about half the 450 names are debugging switches).
- `SafeMode.cfg` documents the **minimum-quality values of the 112 most important settings**: a ready list of what a "safe mode" must reset (windowed 1024 × 768 at 4:3 with `gfx_fAspectRatio` 1.3333, no anti-aliasing, no HDR, no refractions, anisotropy 1, texture and light-map quality 0, 20 active sounds, no ragdolls, subtitles and voices on).
- One safe-mode name, **`sha_bAllowOffsetMapping`**, appears in **no** module, while `Shaders.dll` and `Sam2Game.dll` contain `sha_bNoOffsetMapping` (the opposite sense): the script is stale for that setting (unknown names are presumably ignored or reported; the data does not say which).

## Commands (functions)

The modules export **plain functions** with a lowercase module prefix. They are the console's command vocabulary (the editor's console line "cycles through commands that start with typed letters" and keeps a history: `help-digest.md`). Console-callability is **inferred** from the naming convention and from `svcStartService(...)` being used in scripts; the signatures are not recoverable from names alone. Groups (names only):

| Prefix | Examples (all exist as names) | Purpose |
|---|---|---|
| `svc…` (23) | `svcStartService`, `svcStopService`, `svcRestartService`, `svcEnumerateServices`, `svcFindService`, `svcIsServiceStarted`, `svcChangeRunLevel`, `svcGetCurrentRunLevel`, `svcDisableService` | the **service manager**: named services with **run levels** and priorities, started and stopped from scripts |
| `con…` (38) | `conPrintF`, `conInfoF`, `conWarningF`, `conErrorF`, `conFatalErrorF`, `conDebugF`, `conTraceF`, `conLogF`, `conOpenLog`, `conBackupLog`, `conSetOutputFilter`, `conGetLine`, `conGetLinesCount`, `conAddLineCallback` | console output with **message types** (print, info, warning, error, fatal, debug, trace, log), a log file with backup, an output filter, line callbacks for UIs |
| `dbg…` | `dbgDumpEntityState`, `dbgDumpMemoryInfo`, `dbgDumpReferences`, `dbgDumpScriptStack`, `dbgDumpIDENTs`, `dbgShowMemUsage`, `dbgMakeFatalError` | diagnostics (dumps, forced faults for testing) |
| `prf…` (13) | `prfDumpProfile`, `prfReset`, `prfReportResults`, `prfCalcFPS`, `prfEnableExternalProfiler` | profiler control (`prfDumpProfile` works only when `prf_bShowProfiler` is on: the module's own message) |
| `sim…` (21) | `simPause`, `simUnPause`, `simRequestPause`, `simIsPaused`, `simJogForward`, `simGetStep`, `simNow`, `simLoad_t`, `simSave_t`, `simStart_t`, `simStop`, `simChangeWorld` | simulation control: pause, single-step ("jog"), load/save a world |
| `dem…` (3) | `demStartDemoRecording`, `demStopDemoRecording`, `demIsRecordingDemo` | **demo recording** (replay files; `sam_strDemoFile`, `sam_bLoopDemo`, `sam_bStartDemoPlayback`) |
| `sam…` | `samNextMap`, `samRestartMap`, `samRespawnInPlace`, `samKickPlayer`, `samKickClient`, `samBanPlayer`, `samBanClient`, `samBanIP`, `samTimeBan…`, `samUnBan…`, `samShowBanList`, `samIsCoop`, `samIsDeathmatch`, `samIsSinglePlayer`, `samIsMultiplayer`, `samSetGiveAllWeaponsFlags`, `samTestGfxPerformance`, `samGetModName`, `samGetModVersion`, `samGetVersion` | **server administration** (kick, ban by player, client or address, timed ban, ban list), map flow, game-type queries, a cheat-flag setter, version queries; plus many `samDbg…` drawing helpers (debug lines, boxes and text in the world) |
| `ser…` (8) | `serKickClient`, `serBanClient`, `serBanIP`, `serTimeBanClient`, `serUnBanClient`, `serShowBanList` | the same ban and kick set at the lower (engine) layer |
| `inp…` | `inpGetControlByName`, `inpGetControlName`, `inpGetControlText`, `inpGetControlValue`, `inpGetDevicesCount`, `inpGetDeviceByName`, `inpEnableDevice`, `inpDisableDevice`, `inpStartExclusiveMode`, `inpSubstituteCommands`, `inpUsingGamepad`, `inpLoadVibroEffects`, `inpPollValues`, `inpResetValues` | the **input layer**: devices have named **controls** with a value and a display text; `inpSubstituteCommands` suggests that bound text containing command names is expanded at poll time |
| `net…` | `netIsServer`, `netIsClient`, `netIsHost`, `netIsLocal`, `netIsRemote`, `netIsDemoPlayback`, `netGetServerName`, `netStep` | session role queries |
| `onl…` (100) | online (Steam and lobby) functions | out of scope (`T08`, online services) |

The **admin** set (kick/ban/vote) and the **cheat level** `sam_iEnableCheats` are the multiplayer-relevant commands; the full cheat list is in `T09-028`'s scope (cheat commands) and is not in the data: no cheat names appear in any script.

## Remote console (RCON)

The engine includes a **remote console**: a client and server that exchange command text. The shipped scripts start it through services:

- **`Service.RConServer_Local`** and **`Service.RConClient_Local`**: an in-process transport; `autoexec.cfg` starts both "to allow entering console commands" (this is what makes the in-game console usable).
- **`Service.RConServer_File`**: a **file-based transport**, started by the Xbox script ("allows rcon from PC"); the file names are patterns of the form `<prefix>_C2S.rcon` (client to server), `<prefix>_S2C.rcon` (server to client), `NewClient.rcon` and `ServerActive.rcon`.
- **`Service.RConServer_Telnet`**: a **telnet** server; named in `DedicatedServer.exe`, which is how a dedicated server is administered remotely.
- Variables: `rcon_slMaxBufferSize` (4096 on the Xbox, 0 in stealth mode: no buffer), `rcts_bLocalMode`, `rcts_strAdminPassword` (empty in `user.cfg`, i.e. **no password** by default), `rcts_strWelcomeNote`, `rcts_bDebugNegotiations`.
- Classes in the module: base, local, file and telnet protocols and sessions, an input line with history, a command list, a buffer.

For a reimplementation: a **console with history and tab completion** is needed for development (T03-015) and a **local** transport is enough; a network RCON is optional (the dedicated server could expose an authenticated one later; T08-020).

## Profiles

- A **player profile** is a saved set of per-player settings. The profiles live in a **`PlayerProfiles/` folder** (a string of `Sam2Game.dll`), each presumably as **`Profile.dat`** (strings `/PlayerProfiles/` in `Sam2Game.dll`, `/Profile.dat` in `Engine.dll`); the folder is created at run time under the user's data area and is **not part of the install**. `plp_iMaxPlayerProfiles` limits their number (**4** on the Xbox; the PC value is not in the data), `plp_strActiveProfile` is the active one (`<noprofile>` is the display string when none is selected), `plp_bClearAchievementProgress` is a reset switch.
- **Screens**: profile selection (a scroll list), "new profile" (a name entry), "change name", and the **profile settings** menu with five links: **change name**, **keyboard & mouse controls** (`CMSCustomizeKeys`, the key table), **controller controls**, **advanced options**, **multiplayer characters** (which character a player uses in deathmatch).
- **What a profile stores** (from the settings screens, `menus-tables.md`; the file layout is unknown):

| Setting (screen text) | Values |
|---|---|
| Sensitivity (slider), dead zone of the controller (slider) | numbers |
| Invert look | yes / no |
| Mouse smoothing | yes / no |
| Auto aiming; controller auto aim | yes / no |
| Auto-centre view | yes / no |
| Auto weapon change | yes / no / **only if stronger** |
| Use auto jump | yes / no (`player-rules.md`: on in single player, off in multiplayer) |
| View bobbing | yes / no |
| Prefer 3rd-person view | no / yes |
| Flip fire buttons | yes / no |
| Weapons list | horizontal / vertical (the names `sam_aiWeaponsMenuDirAndOrder`, `samGetWeaponsMenuDir/Order` probably belong to it) |
| Crosshair colour | white, red, green, blue, yellow, cyan, magenta, black |
| Crosshair texture | 18 choices (0 to 17); crosshair size (slider) |
| Puppet radar | no, right top, right middle, right bottom, center bottom, left middle; radar offset Y (slider) |
| Controller **axis controls** preset | standard, lefty, beginner, lefty beginner |
| Controller **button controls** preset | custom, default, standard, standard A, alternative, lefty, standard B, alternative B |

- **Single-player settings** screen (a game set-up, not stored in the profile): difficulty (Tourist, Easy, Normal, Hard, Serious; `player-rules.md`), **use lives**, **use combo weapons**, **enable sprinting**, **enable rocket jumping**, **enemy multiplier** (none, 2× to 11×) and **enemy max multiplier** (50 % to 1000 %: 18 steps); a warning says these "may have detrimental effects on performance and gameplay". These are mod-like options of the retail game, to be reproduced as game-rule flags (`samIsUsingLives` exists as a query).

## Key bindings

- **The default key assignments are not in the install.** The custom-keys gadget (`CMenuGadgetCustomKeysLayout`, in `CMSCustomizeKeys` and `CMSCustomizeButtons`) only has an up/down scroll and a caption: the **action list and the default devices and keys come from code**, and the chosen bindings are written to the profile. The console prints `"%1 rebound to %2 for profile %3"` after a rebinding (action, control, profile).
- **Bindings are per profile and per device.** The input layer names every control of every device (keyboard, mouse, gamepads; `inpGetControlName`, `inpGetControlText`), and the name of `men_bShowHiddenKeys` suggests a switch that shows controls the menu normally hides.
- **Gamepad**: the controller screen shows a picture of a pad with labels on **Start, Back, Black, White, Red, Green, Blue, Yellow, D-pad (4), left and right thumb and trigger, and left and right trigger arrows** (the Xbox pad: the *colour* buttons are the A, B, X, Y buttons, Black and White are the extra buttons). Presets are chosen by name (lists above); `Custom` means "use the custom table".
- **Vibration** is a separate effect set (`fx.md`: `.vfx`).
- A reimplementation therefore has to define its **own default bindings** (T03-005 input state; T10 menus). For authenticity the *action vocabulary* can be taken from the game's player controls (`weapons.md`, `player-rules.md`: move, strafe, jump, crouch/duck, fire, alt-fire, weapon select, use, look, zoom) and the preset *names* above; **the exact default keys of Serious Sam 2 remain unspecified** (T01-078, oracle capture T01-038 can record them from a running original).

## What is not in the data

1. The default key and button bindings (code), the layout of `Profile.dat`, and the value of `plp_iMaxPlayerProfiles` on PC.
2. The order and precedence of `boot.cfg`, `autoexec.cfg`, `user.cfg` and the `.ini` files, and whether the `.ini` files exist at all in a normal install.
3. Default values of most of the 450 variables (code); only the 112 safe-mode values are known.
4. Which exported functions are callable from the console, their arguments, and the cheat command names.
5. The wire format of the remote console (out of scope).
6. What an unknown variable name in a script does.

## Reader invariants

1. A script is a list of sections; a statement is `name = value;`, a call `fn(args);`, or an `if … else …` block; `//` starts a comment; a float may end with `f`; strings are double-quoted; whitespace and line ends (CRLF or LF) are insignificant.
2. An assignment to an unknown name is **not an error that stops the game** (the stale `sha_bAllowOffsetMapping` line is in a shipped script): log and continue.
3. The mod selection is `CURRENTMOD = <file> ;` from `Content/CurrentMod.txt`, else `Content/DefaultMod.txt` (`SeriousSam2.mod`).
4. Profiles hold **per-player** options; game rules (difficulty, lives, combo weapons, sprinting, rocket jumping, enemy multipliers) belong to the **game set-up**.

## Test vectors

- `SafeMode.cfg`: 112 assignments; `gfx_pixResWidth = 1024;`, `gfx_pixResHeight = 768;`, `ren_fNearClipPlaneDistance = 0.1;`, `snd_fMovieVolume = 1;`.
- `user.cfg` = `[Engine] ` + `rcts_strAdminPassword=""` (35 bytes).
- `autoexec.cfg`: `[Engine]` with the two `svcStartService(ident("Service.RConServer_Local" / "Service.RConClient_Local"))` calls (192 bytes).
- `autoexec_xbox.cfg` assignments: `rcon_slMaxBufferSize = 4096;`, `crm_fDistanceBias = 0.8f;`, `crm_fDensityBias = 0.6f;`, `plp_iMaxPlayerProfiles = 4;`, `inp_fYLookSensitivity=0.75;`.
- `DefaultMod.txt` ends with `CURRENTMOD = SeriousSam2.mod ;`.
- Name counts: **450** names in **40** families (`console-variables.md`); service names `Service.RConServer_Local`, `Service.RConClient_Local`, `Service.RConServer_File`, `Service.RConServer_Telnet`.
